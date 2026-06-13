//! Resolves `prose` configuration from `prose.toml` or the
//! `[tool.prose]` table of `pyproject.toml`.
//!
//! `Config::load` walks upward from a starting path toward the
//! filesystem root. In each directory a `prose.toml` outranks a
//! `pyproject.toml`, and the nearest directory carrying either wins.
//! A `prose.toml` holds the config at its document root, whereas a
//! `pyproject.toml` nests it under `[tool.prose]`. Reaching the root
//! without a match resolves to full defaults, so prose works on a
//! fresh project with no configuration step.
//!
//! Each rule's configuration lives under `[tool.prose.rules]`, where
//! a bare bool toggles the rule and a sub-table carries its knobs.

use std::{num::NonZeroUsize, path::Path};

use ruff_python_ast::PythonVersion;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::rule::RuleConfigs;

mod de;
mod load;
mod schema;

use de::deserialize_import_line_length;
pub(crate) use de::deserialize_rule;
use load::{
    ConfigNotice, emit_notice, parse_prose_toml, parse_pyproject, pyproject_declares_prose,
    read_optional,
};
pub use schema::*;

/// Filename of the dedicated config, parsed with its keys at the
/// document root.
const PROSE_TOML: &str = "prose.toml";

/// Filename of the shared manifest, parsed under its `[tool.prose]`
/// table.
const PYPROJECT_TOML: &str = "pyproject.toml";

/// The resolved `prose` configuration, read from a `prose.toml` root
/// or a `pyproject.toml` `[tool.prose]` table.
///
/// `code_line_length` defaults to `Some(88)`. `docstring_line_length`
/// defaults to `Some(76)`. `import_line_length` defaults to `Some(120)`,
/// falling back to `code_line_length` when `false`.
/// `docstring_structured_policy` defaults to `CodeLineLength`.
/// `imports.first_party` defaults to empty. `target_version` defaults
/// to `None`. Per-rule settings live under `rules`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub cache: CacheConfig,
    pub code_line_length: Option<NonZeroUsize>,
    pub docstring_line_length: Option<NonZeroUsize>,
    pub docstring_structured_policy: DocstringStructuredPolicy,
    #[serde(deserialize_with = "deserialize_import_line_length")]
    pub import_line_length: Option<NonZeroUsize>,
    pub imports: ImportsConfig,
    pub rules: RuleConfigs,
    pub target_version: Option<PythonVersion>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
            code_line_length: NonZeroUsize::new(88),
            docstring_line_length: NonZeroUsize::new(76),
            docstring_structured_policy: DocstringStructuredPolicy::default(),
            import_line_length: NonZeroUsize::new(120),
            imports: ImportsConfig::default(),
            rules: RuleConfigs::default(),
            target_version: None,
        }
    }
}

impl Config {
    /// Parses a `prose.toml` snippet directly from a string, reading
    /// its keys at the document root.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Toml` when `contents` is not valid TOML.
    pub fn from_prose_toml_str(contents: &str) -> Result<Self, ConfigError> {
        parse_prose_toml(contents, &mut emit_notice)
    }

    /// Parses a `pyproject.toml` snippet directly from a string.
    ///
    /// Returns `Config::default()` when `contents` carries no
    /// `[tool.prose]` section. Unknown keys under `[tool.prose]` warn
    /// to stderr, mirroring [`Config::load`].
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Toml` when `contents` is not valid TOML.
    pub fn from_pyproject_str(contents: &str) -> Result<Self, ConfigError> {
        Ok(parse_pyproject(contents, &mut emit_notice)?.unwrap_or_default())
    }

    /// Walks upward from `from`, returning the config from the nearest
    /// directory that carries a `prose.toml` or a `pyproject.toml` with
    /// a `[tool.prose]` table, or `Config::default()` if neither exists
    /// on the chain. A `prose.toml` outranks a same-directory
    /// `pyproject.toml`.
    ///
    /// Unknown keys and the precedence outcome are logged to stderr and
    /// ignored, keeping the loader forward-compatible with rules added
    /// in future releases.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Io` if a config file is found but cannot be
    /// read, and `ConfigError::Toml` if its contents are not valid TOML.
    pub fn load<P: AsRef<Path>>(from: P) -> Result<Self, ConfigError> {
        Self::load_with_notices(from, emit_notice)
    }

    /// Shared implementation backing `load`, factored out so tests can
    /// inspect the emitted notices without capturing stderr.
    fn load_with_notices<P, F>(from: P, mut on_notice: F) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
        F: FnMut(ConfigNotice<'_>),
    {
        for dir in from.as_ref().ancestors() {
            if let Some(contents) = read_optional(dir.join(PROSE_TOML))? {
                if pyproject_declares_prose(dir) {
                    on_notice(ConfigNotice::ProseTomlPrecedence(dir));
                }
                return parse_prose_toml(&contents, &mut on_notice);
            }
            if let Some(contents) = read_optional(dir.join(PYPROJECT_TOML))?
                && let Some(config) = parse_pyproject(&contents, &mut on_notice)?
            {
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    pub(crate) fn code_width(&self) -> usize {
        self.code_line_length
            .expect("Config::default synthesizes Some(88)")
            .get()
    }

    pub(crate) fn docstring_width(&self) -> usize {
        self.docstring_line_length
            .expect("Config::default synthesizes Some(76)")
            .get()
    }

    pub(crate) fn first_party(&self) -> Vec<String> {
        self.imports.first_party.clone()
    }

    /// The budget governing import wrapping, falling back to the code
    /// budget when `import_line_length` is `None`.
    pub(crate) fn import_width(&self) -> usize {
        self.import_line_length
            .map_or_else(|| self.code_width(), NonZeroUsize::get)
    }
}

/// Failure to load a `prose` configuration from a `prose.toml` or a
/// `pyproject.toml`.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;
    use crate::testing::{write_prose_toml, write_pyproject};

    fn assert_toml_error(toml: &str) {
        assert_matches!(Config::from_pyproject_str(toml), Err(ConfigError::Toml(_)));
    }

    #[test]
    fn docstring_line_length_defaults_to_76_when_field_absent() {
        let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

        assert_eq!(config.docstring_line_length, NonZeroUsize::new(76));
    }

    #[test]
    fn docstring_line_length_explicit_override_takes_effect() {
        let config = Config::from_pyproject_str("[tool.prose]\ndocstring-line-length = 100\n")
            .expect("parses");

        assert_eq!(config.docstring_line_length, NonZeroUsize::new(100));
    }

    #[test]
    fn docstring_line_length_negative_returns_toml_error() {
        assert_toml_error("[tool.prose]\ndocstring-line-length = -1\n");
    }

    #[test]
    fn docstring_line_length_zero_returns_toml_error() {
        assert_toml_error("[tool.prose]\ndocstring-line-length = 0\n");
    }

    #[test]
    fn docstring_structured_policy_defaults_to_code_line_length_when_field_absent() {
        let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

        assert_eq!(
            config.docstring_structured_policy,
            DocstringStructuredPolicy::CodeLineLength
        );
    }

    #[test]
    fn docstring_structured_policy_explicit_override_to_docstring_line_length() {
        let config = Config::from_pyproject_str(
            "[tool.prose]\ndocstring-structured-policy = \"docstring-line-length\"\n",
        )
        .expect("parses");

        assert_eq!(
            config.docstring_structured_policy,
            DocstringStructuredPolicy::DocstringLineLength
        );
    }

    #[test]
    fn docstring_structured_policy_invalid_value_returns_toml_error() {
        assert_toml_error("[tool.prose]\ndocstring-structured-policy = \"nonsense\"\n");
    }

    #[test]
    fn from_prose_toml_str_empty_returns_defaults() {
        let config = Config::from_prose_toml_str("").expect("parses");

        assert_eq!(config.code_line_length, NonZeroUsize::new(88));
        assert!(config.rules.align_equals.enabled);
    }

    #[test]
    fn from_prose_toml_str_reads_bare_root_keys() {
        let config =
            Config::from_prose_toml_str("code-line-length = 120\n[rules]\nalphabetize = false\n")
                .expect("parses");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
        assert!(!config.rules.alphabetize.enabled);
    }

    #[test]
    fn from_pyproject_str_with_unknown_key_warns_and_returns_config() {
        let config = Config::from_pyproject_str(
            "[tool.prose]\ncode-line-length = 100\nunknown-future-key = 1\n",
        )
        .expect("parses");

        assert_eq!(config.code_line_length, NonZeroUsize::new(100));
    }

    #[test]
    fn import_line_length_defaults_to_120_when_field_absent() {
        let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

        assert_eq!(config.import_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn import_line_length_explicit_override_takes_effect() {
        let config =
            Config::from_pyproject_str("[tool.prose]\nimport-line-length = 100\n").expect("parses");

        assert_eq!(config.import_line_length, NonZeroUsize::new(100));
    }

    #[test]
    fn import_line_length_false_falls_back_to_code_line_length() {
        let config = Config::from_pyproject_str("[tool.prose]\nimport-line-length = false\n")
            .expect("parses");

        assert!(config.import_line_length.is_none());
        assert_eq!(config.import_width(), config.code_width());
    }

    #[test]
    fn import_line_length_negative_returns_toml_error() {
        assert_toml_error("[tool.prose]\nimport-line-length = -1\n");
    }

    #[test]
    fn import_line_length_true_returns_toml_error() {
        assert_toml_error("[tool.prose]\nimport-line-length = true\n");
    }

    #[test]
    fn import_line_length_zero_returns_toml_error() {
        assert_toml_error("[tool.prose]\nimport-line-length = 0\n");
    }

    #[test]
    fn import_width_uses_import_line_length_when_set() {
        let config =
            Config::from_pyproject_str("[tool.prose]\nimport-line-length = 100\n").expect("parses");

        assert_eq!(config.import_width(), 100);
    }

    #[test]
    fn imports_first_party_defaults_to_empty_when_absent() {
        let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

        assert!(config.imports.first_party.is_empty());
    }

    #[test]
    fn imports_first_party_reads_kebab_case_list() {
        let config = Config::from_pyproject_str(
            "[tool.prose.imports]\nfirst-party = [\"myapp\", \"acme\"]\n",
        )
        .expect("parses");

        assert_eq!(config.imports.first_party, ["myapp", "acme"]);
    }

    #[test]
    fn load_absent_file_returns_defaults() {
        let tmp = TempDir::new().expect("tempdir");
        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(88));
        assert!(config.rules.align_equals.enabled);
    }

    #[test]
    fn load_absent_prose_section_returns_defaults() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[project]\nname = \"x\"\n");

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(88));
        assert!(config.rules.align_equals.enabled);
    }

    #[test]
    fn load_emits_precedence_notice_when_both_present() {
        let tmp = TempDir::new().expect("tempdir");
        write_prose_toml(tmp.path(), "code-line-length = 120\n");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

        let mut precedence_notices = 0;
        let config = Config::load_with_notices(tmp.path(), |notice| {
            if matches!(notice, ConfigNotice::ProseTomlPrecedence(_)) {
                precedence_notices += 1;
            }
        })
        .expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
        assert_eq!(precedence_notices, 1);
    }

    #[test]
    fn load_empty_prose_toml_returns_defaults_and_stops_walk() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("child");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_prose_toml(&nested, "");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(88));
    }

    #[test]
    fn load_from_a_file_path_walks_from_its_directory() {
        let tmp = TempDir::new().expect("tempdir");
        write_prose_toml(tmp.path(), "code-line-length = 120\n");
        let file = tmp.path().join("mod.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let config = Config::load(&file).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_malformed_toml_returns_toml_error() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[this is not valid TOML");

        let result = Config::load(tmp.path());

        assert_matches!(result, Err(ConfigError::Toml(_)));
    }

    #[test]
    fn load_partial_override_preserves_other_rule_defaults() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose.rules.align-equals]\nenabled = false\n",
        );

        let config = Config::load(tmp.path()).expect("loads");

        assert!(!config.rules.align_equals.enabled);
        assert!(config.rules.align_colons.enabled);
        assert!(config.rules.strip_trailing_commas.enabled);
    }

    #[test]
    fn load_per_rule_max_shift_overrides_are_independent() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            indoc! {r#"
                [tool.prose.rules.align-colons]
                max-shift = false

                [tool.prose.rules.align-equals]
                max-shift = 0
            "#},
        );

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.rules.align_colons.max_shift, MaxShift::Unlimited);
        assert_eq!(config.rules.align_equals.max_shift, MaxShift::NoShift);
    }

    #[test]
    fn load_picks_nearest_ancestor_when_multiple_configs_exist() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a/b");
        std::fs::create_dir_all(&nested).expect("nested dirs create");

        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");
        write_pyproject(&nested, "[tool.prose]\ncode-line-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_picks_nearest_prose_toml_over_ancestor_pyproject() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("child");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");
        write_prose_toml(&nested, "code-line-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_prefers_prose_toml_over_sibling_pyproject() {
        let tmp = TempDir::new().expect("tempdir");
        write_prose_toml(tmp.path(), "code-line-length = 120\n");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 80\n");

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_reads_pure_prose_toml() {
        let tmp = TempDir::new().expect("tempdir");
        write_prose_toml(tmp.path(), "code-line-length = 120\n");

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_unknown_key_invokes_callback() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\nunknown-future-key = \"whatever\"\n",
        );

        let mut captured = Vec::new();
        let config = Config::load_with_notices(tmp.path(), |notice| {
            if let ConfigNotice::UnknownKey(key) = notice {
                captured.push(key.to_owned());
            }
        })
        .expect("loads");

        assert_eq!(captured, ["unknown-future-key"]);
        assert!(config.rules.align_equals.enabled);
    }

    #[test]
    fn load_unknown_key_routes_through_default_warn_callback() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\nunknown-future-key = \"x\"\n");

        let config = Config::load(tmp.path()).expect("loads");

        assert!(config.rules.align_equals.enabled);
    }

    #[test]
    fn load_unreadable_config_returns_io_error() {
        let tmp = TempDir::new().expect("tempdir");
        std::fs::create_dir(tmp.path().join("prose.toml")).expect("dir at config path");

        assert_matches!(Config::load(tmp.path()), Err(ConfigError::Io(_)));
    }

    #[test]
    fn load_walks_past_sectionless_pyproject_to_ancestor_prose_toml() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("child");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_pyproject(&nested, "[project]\nname = \"x\"\n");
        write_prose_toml(tmp.path(), "code-line-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_walks_up_to_ancestor_directory() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn max_atomics_per_line_explicit_integer_takes_effect() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.collection-layout]\nmax-atomics-per-line = 3\n",
        )
        .expect("parses");

        assert_eq!(
            config.rules.collection_layout.max_atomics_per_line,
            NonZeroUsize::new(3),
        );
    }

    #[test]
    fn max_atomics_per_line_false_disables_cap() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.collection-layout]\nmax-atomics-per-line = false\n",
        )
        .expect("parses");

        assert!(
            config
                .rules
                .collection_layout
                .max_atomics_per_line
                .is_none()
        );
    }

    #[test]
    fn max_atomics_per_line_true_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.collection-layout]\nmax-atomics-per-line = true\n");
    }

    #[test]
    fn max_inline_dict_entries_explicit_integer_takes_effect() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = 5\n",
        )
        .expect("parses");

        assert_eq!(
            config.rules.collection_layout.max_inline_dict_entries,
            NonZeroUsize::new(5),
        );
    }

    #[test]
    fn max_inline_dict_entries_false_disables_count_trigger() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = false\n",
        )
        .expect("parses");

        assert!(
            config
                .rules
                .collection_layout
                .max_inline_dict_entries
                .is_none()
        );
    }

    #[test]
    fn max_inline_dict_entries_true_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = true\n");
    }

    #[test]
    fn max_inline_dict_entries_zero_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.collection-layout]\nmax-inline-dict-entries = 0\n");
    }

    #[test]
    fn max_inline_params_explicit_integer_takes_effect() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.signature-layout]\nmax-inline-params = 5\n",
        )
        .expect("parses");

        assert_eq!(
            config.rules.signature_layout.max_inline_params,
            NonZeroUsize::new(5),
        );
    }

    #[test]
    fn max_inline_params_false_disables_count_trigger() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.signature-layout]\nmax-inline-params = false\n",
        )
        .expect("parses");

        assert!(config.rules.signature_layout.max_inline_params.is_none());
    }

    #[test]
    fn max_inline_params_string_value_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.signature-layout]\nmax-inline-params = \"off\"\n");
    }

    #[test]
    fn max_inline_params_true_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.signature-layout]\nmax-inline-params = true\n");
    }

    #[test]
    fn max_inline_params_zero_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.signature-layout]\nmax-inline-params = 0\n");
    }

    #[test]
    fn max_shift_default_is_sixteen() {
        let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

        assert_eq!(
            config.rules.align_equals.max_shift,
            MaxShift::Cap(NonZeroUsize::new(16).expect("16 is non-zero")),
        );
    }

    #[test]
    fn max_shift_negative_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.align-equals]\nmax-shift = -1\n");
    }

    #[rstest]
    #[case("4", MaxShift::Cap(NonZeroUsize::new(4).expect("4 is non-zero")))]
    #[case("false", MaxShift::Unlimited)]
    #[case("0", MaxShift::NoShift)]
    fn max_shift_reads_each_value_form(#[case] value: &str, #[case] expected: MaxShift) {
        let config = Config::from_pyproject_str(&format!(
            "[tool.prose.rules.align-equals]\nmax-shift = {value}\n"
        ))
        .expect("parses");

        assert_eq!(config.rules.align_equals.max_shift, expected);
    }

    #[rstest]
    #[case("0")]
    #[case("4")]
    #[case("false")]
    fn max_shift_round_trips_through_toml(#[case] value: &str) {
        let config = Config::from_pyproject_str(&format!(
            "[tool.prose.rules.align-equals]\nmax-shift = {value}\n"
        ))
        .expect("parses");
        let dumped = toml::to_string(&config).expect("Config serializes");
        let reparsed = Config::from_prose_toml_str(&dumped).expect("reparses");

        assert_eq!(
            reparsed.rules.align_equals.max_shift,
            config.rules.align_equals.max_shift,
        );
    }

    #[test]
    fn max_shift_true_returns_toml_error() {
        assert_toml_error("[tool.prose.rules.align-equals]\nmax-shift = true\n");
    }

    #[test]
    fn rules_bare_bool_false_leaves_other_knobs_default() {
        let config = Config::from_pyproject_str("[tool.prose.rules]\nalphabetize = false\n")
            .expect("parses");

        assert!(!config.rules.alphabetize.enabled);
        assert!(config.rules.alphabetize.docstring_entries);
    }

    #[rstest]
    #[case("false", false)]
    #[case("true", true)]
    fn rules_bare_bool_sets_enabled(#[case] literal: &str, #[case] expected: bool) {
        let config =
            Config::from_pyproject_str(&format!("[tool.prose.rules]\nalphabetize = {literal}\n"))
                .expect("parses");

        assert_eq!(config.rules.alphabetize.enabled, expected);
        assert!(config.rules.align_equals.enabled);
    }

    #[test]
    fn rules_inline_table_compiles_regex_knob() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules]\nsingle-use-variables = { allow-pattern = \"^tmp_\" }\n",
        )
        .expect("parses");

        assert!(
            config
                .rules
                .single_use_variables
                .allow_pattern
                .is_match("tmp_x")
        );
        assert!(config.rules.single_use_variables.enabled);
    }

    #[test]
    fn rules_inline_table_resolves_nested_max_inline_params() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules]\nsignature-layout = { max-inline-params = false }\n",
        )
        .expect("parses");

        assert!(config.rules.signature_layout.enabled);
        assert!(config.rules.signature_layout.max_inline_params.is_none());
    }

    #[test]
    fn rules_inline_table_sets_knob_and_stays_enabled() {
        let config =
            Config::from_pyproject_str("[tool.prose.rules]\nalign-equals = { max-shift = 4 }\n")
                .expect("parses");

        assert!(config.rules.align_equals.enabled);
        assert_eq!(
            config.rules.align_equals.max_shift,
            MaxShift::Cap(NonZeroUsize::new(4).expect("4 is non-zero")),
        );
    }

    #[test]
    fn rules_non_bool_non_table_value_returns_toml_error() {
        assert_toml_error("[tool.prose.rules]\nalign-equals = 5\n");
    }

    #[test]
    fn rules_subtable_form_still_parses() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.align-equals]\nenabled = false\nmax-shift = 4\n",
        )
        .expect("parses");

        assert!(!config.rules.align_equals.enabled);
        assert_eq!(
            config.rules.align_equals.max_shift,
            MaxShift::Cap(NonZeroUsize::new(4).expect("4 is non-zero")),
        );
    }

    #[test]
    fn rules_unknown_subtable_key_invokes_notice() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose.rules.align-equals]\nbogus-knob = 1\n",
        );

        let mut captured = Vec::new();
        Config::load_with_notices(tmp.path(), |notice| {
            if let ConfigNotice::UnknownKey(key) = notice {
                captured.push(key.to_owned());
            }
        })
        .expect("loads");

        assert_eq!(captured, ["rules.align-equals.bogus-knob"]);
    }

    #[test]
    fn single_use_variables_explicit_allow_pattern_takes_effect() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules.single-use-variables]\nallow-pattern = \"^tmp_\"\n",
        )
        .expect("parses");

        assert!(
            config
                .rules
                .single_use_variables
                .allow_pattern
                .is_match("tmp_x")
        );
        assert!(
            !config
                .rules
                .single_use_variables
                .allow_pattern
                .is_match("xtmp_")
        );
    }

    #[test]
    fn single_use_variables_invalid_allow_pattern_returns_toml_error() {
        assert_toml_error(
            "[tool.prose.rules.single-use-variables]\nallow-pattern = \"[unclosed\"\n",
        );
    }

    #[test]
    fn target_version_accepts_unrecognized_minor() {
        let config = Config::from_pyproject_str("[tool.prose]\ntarget-version = \"3.99\"\n")
            .expect("parses");

        assert_eq!(
            config.target_version,
            Some(PythonVersion {
                major: 3,
                minor: 99
            })
        );
    }

    #[test]
    fn target_version_defaults_to_none_when_field_absent() {
        let config = Config::from_pyproject_str("[tool.prose]\n").expect("parses");

        assert_eq!(config.target_version, None);
    }

    #[test]
    fn target_version_every_variant_round_trips_through_serde() {
        for version in PythonVersion::iter() {
            let toml = format!("[tool.prose]\ntarget-version = \"{version}\"\n");
            let config = Config::from_pyproject_str(&toml).expect("parses");

            assert_eq!(config.target_version, Some(version));
        }
    }

    #[test]
    fn target_version_extra_period_returns_toml_error() {
        assert_toml_error("[tool.prose]\ntarget-version = \"3.14.0\"\n");
    }

    #[test]
    fn target_version_invalid_value_returns_toml_error() {
        assert_toml_error("[tool.prose]\ntarget-version = \"py310\"\n");
    }
}

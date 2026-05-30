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

use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use regex_lite::Regex;
use ruff_python_ast::PythonVersion;
use serde::de::{value::MapAccessDeserializer, IntoDeserializer, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

pub use crate::rule::RuleConfigs;

/// Filename of the dedicated config, parsed with its keys at the
/// document root.
const PROSE_TOML: &str = "prose.toml";

/// Filename of the shared manifest, parsed under its `[tool.prose]`
/// table.
const PYPROJECT_TOML: &str = "pyproject.toml";

/// Configuration shared by the alignment rules (`align_colons`, `align_equals`).
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlignmentConfig {
    pub enabled: bool,
    pub max_shift: NonZeroUsize,
    pub max_shift_policy: MaxAlignShiftPolicy,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_shift: NonZeroUsize::new(8).expect("8 is non-zero"),
            max_shift_policy: MaxAlignShiftPolicy::default(),
        }
    }
}

/// Configuration for the `alphabetize` rule. `docstring_entries`
/// gates the Google-style entry-section reorder pass, leaving the
/// AST-level sorts to apply on their own when set `false`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AlphabetizeConfig {
    pub docstring_entries: bool,
    pub enabled: bool,
}

impl Default for AlphabetizeConfig {
    fn default() -> Self {
        Self {
            docstring_entries: true,
            enabled: true,
        }
    }
}

/// Configuration for the `bare_import_allowlist` rule.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BareImportAllowlistConfig {
    pub allow: Vec<String>,
    pub enabled: bool,
}

impl Default for BareImportAllowlistConfig {
    fn default() -> Self {
        Self {
            allow: vec!["numpy".to_owned(), "pandas".to_owned()],
            enabled: true,
        }
    }
}

/// Cache settings parsed from `[tool.prose.cache]`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size_mib: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mib: 100,
        }
    }
}

/// Configuration for the `collection_layout` rule.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollectionLayoutConfig {
    pub enabled: bool,
    pub max_atomics_per_line: Option<NonZeroUsize>,
}

impl Default for CollectionLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_atomics_per_line: NonZeroUsize::new(8),
        }
    }
}

/// The resolved `prose` configuration, read from a `prose.toml` root
/// or a `pyproject.toml` `[tool.prose]` table.
///
/// `code_line_length` defaults to `Some(88)`. `docstring_line_length`
/// defaults to `Some(76)`. `docstring_structured_policy` defaults
/// to `CodeLineLength`. `imports.first_party` defaults to empty.
/// `target_version` defaults to `None`. Per-rule settings live under
/// `rules`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub cache: CacheConfig,
    pub code_line_length: Option<NonZeroUsize>,
    pub docstring_line_length: Option<NonZeroUsize>,
    pub docstring_structured_policy: DocstringStructuredPolicy,
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
            if let Some(contents) = read_optional(dir.join(PYPROJECT_TOML))? {
                if let Some(config) = parse_pyproject(&contents, &mut on_notice)? {
                    return Ok(config);
                }
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

/// Which budget structured docstring sections wrap to.
///
/// `CodeLineLength` reuses `Config::code_line_length`.
/// `DocstringLineLength` reuses `Config::docstring_line_length`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocstringStructuredPolicy {
    #[default]
    CodeLineLength,
    DocstringLineLength,
}

/// Settings parsed from `[tool.prose.imports]`. `first_party` lists
/// the package names whose imports group with relative imports as
/// local-package, keyed kebab-case under `first-party`.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ImportsConfig {
    pub first_party: Vec<String>,
}

/// Configuration for the `loose_constants` rule.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LooseConstantsConfig {
    pub allow: Vec<String>,
    pub enabled: bool,
}

impl Default for LooseConstantsConfig {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            enabled: true,
        }
    }
}

/// What to do when an alignment group's widest padding exceeds the
/// rule's `max-shift`.
///
/// `Split` greedily partitions the group so each contiguous
/// sub-group satisfies the cap, and each sub-group of size `>= 2`
/// aligns independently. `Drop` excludes the widest member(s) from
/// the padding calculation until the cap is satisfied, leaving those
/// members at their original spacing while neighbors align around
/// them.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaxAlignShiftPolicy {
    Drop,
    #[default]
    Split,
}

/// Configuration for the `signature_layout` rule.
///
/// `max_inline_params` caps the count threshold. A positive integer
/// enforces the cap. `false` disables the count trigger.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SignatureLayoutConfig {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_max_inline_params")]
    pub max_inline_params: Option<NonZeroUsize>,
}

impl Default for SignatureLayoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_inline_params: NonZeroUsize::new(4),
        }
    }
}

/// Configuration for the `single_use_variables` rule.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SingleUseVariablesConfig {
    #[serde(
        deserialize_with = "deserialize_regex",
        serialize_with = "serialize_regex"
    )]
    pub allow_pattern: Regex,
    pub enabled: bool,
}

impl Default for SingleUseVariablesConfig {
    fn default() -> Self {
        Self {
            allow_pattern: Regex::new("^_").expect("`^_` compiles"),
            enabled: true,
        }
    }
}

/// Sub-table shape for rules whose only knob is `enabled`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToggleOnly {
    pub enabled: bool,
}

impl Default for ToggleOnly {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl RuleToggle for ToggleOnly {
    fn with_enabled(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// A per-rule config a bare bool can toggle. `with_enabled` is the
/// shorthand for the `{ enabled = <bool> }` table under
/// `[tool.prose.rules]`, leaving every other knob at its default.
pub(crate) trait RuleToggle: Default {
    fn with_enabled(enabled: bool) -> Self;
}

/// Implements [`RuleToggle`] for configs carrying knobs beyond
/// `enabled`, filling the rest from `Default`.
macro_rules! impl_rule_toggle {
    ($($config:ty),+ $(,)?) => {
        $(impl RuleToggle for $config {
            fn with_enabled(enabled: bool) -> Self {
                Self { enabled, ..Self::default() }
            }
        })+
    };
}

impl_rule_toggle!(
    AlignmentConfig,
    AlphabetizeConfig,
    BareImportAllowlistConfig,
    CollectionLayoutConfig,
    LooseConstantsConfig,
    SignatureLayoutConfig,
    SingleUseVariablesConfig,
);

/// A diagnostic surfaced while resolving configuration.
enum ConfigNotice<'a> {
    /// A `prose.toml` outranked a `[tool.prose]` table in a
    /// `pyproject.toml` sharing its directory. Carries that directory.
    ProseTomlPrecedence(&'a Path),
    /// An unrecognized key under the prose table. Carries the dotted
    /// key path.
    UnknownKey(&'a str),
}

/// Resolves a rule's config from either a bare bool toggle or a
/// sub-table. `deserialize_any` dispatches on the TOML value so the
/// sub-table arm forwards a live map, preserving `serde_ignored`'s
/// unknown-key tracking inside the table.
pub(crate) fn deserialize_rule<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: RuleToggle + Deserialize<'de>,
{
    struct RuleVisitor<T>(PhantomData<T>);

    impl<'de, T: RuleToggle + Deserialize<'de>> Visitor<'de> for RuleVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a boolean toggle or a rule sub-table")
        }

        fn visit_bool<E: serde::de::Error>(self, enabled: bool) -> Result<T, E> {
            Ok(T::with_enabled(enabled))
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(RuleVisitor(PhantomData))
}

fn deserialize_max_inline_params<'de, D>(deserializer: D) -> Result<Option<NonZeroUsize>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Cap(NonZeroUsize),
        Off(bool),
    }
    match Value::deserialize(deserializer)? {
        Value::Cap(n) => Ok(Some(n)),
        Value::Off(false) => Ok(None),
        Value::Off(true) => Err(serde::de::Error::custom(
            "`max-inline-params` accepts a positive integer or `false`, not `true`",
        )),
    }
}

fn deserialize_prose<F>(value: toml::Value, on_notice: &mut F) -> Result<Config, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    Ok(serde_ignored::deserialize(
        value.into_deserializer(),
        |path| {
            on_notice(ConfigNotice::UnknownKey(&path.to_string()));
        },
    )?)
}

fn deserialize_regex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Regex, D::Error> {
    let pattern = String::deserialize(deserializer)?;
    Regex::new(&pattern).map_err(serde::de::Error::custom)
}

fn emit_notice(notice: ConfigNotice<'_>) {
    match notice {
        ConfigNotice::ProseTomlPrecedence(dir) => eprintln!(
            "note: prose.toml takes precedence over the [tool.prose] table in {}",
            dir.join(PYPROJECT_TOML).display(),
        ),
        ConfigNotice::UnknownKey(key) => {
            eprintln!("warning: unknown key `{key}` in [tool.prose]");
        }
    }
}

fn parse_prose_toml<F>(contents: &str, on_notice: &mut F) -> Result<Config, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    deserialize_prose(toml::from_str(contents)?, on_notice)
}

fn parse_pyproject<F>(contents: &str, on_notice: &mut F) -> Result<Option<Config>, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    let value: toml::Value = toml::from_str(contents)?;
    let Some(prose) = prose_table(&value).cloned() else {
        return Ok(None);
    };
    Ok(Some(deserialize_prose(prose, on_notice)?))
}

fn prose_table(value: &toml::Value) -> Option<&toml::Value> {
    value.get("tool").and_then(|tool| tool.get("prose"))
}

fn pyproject_declares_prose(dir: &Path) -> bool {
    fs_err::read_to_string(dir.join(PYPROJECT_TOML))
        .ok()
        .and_then(|contents| toml::from_str::<toml::Value>(&contents).ok())
        .is_some_and(|value| prose_table(&value).is_some())
}

fn read_optional(path: PathBuf) -> Result<Option<String>, ConfigError> {
    match fs_err::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn serialize_regex<S: Serializer>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(regex.as_str())
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    fn assert_toml_error(toml: &str) {
        assert_matches!(Config::from_pyproject_str(toml), Err(ConfigError::Toml(_)));
    }

    fn write_prose_toml(dir: &Path, contents: &str) {
        std::fs::write(dir.join("prose.toml"), contents).expect("prose.toml writes");
    }

    fn write_pyproject(dir: &Path, contents: &str) {
        std::fs::write(dir.join("pyproject.toml"), contents).expect("pyproject writes");
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
    fn load_per_rule_policy_overrides_are_independent() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            indoc! {r#"
                [tool.prose.rules.align-colons]
                max-shift-policy = "drop"

                [tool.prose.rules.align-equals]
                max-shift-policy = "split"
            "#},
        );

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(
            config.rules.align_colons.max_shift_policy,
            MaxAlignShiftPolicy::Drop
        );
        assert_eq!(
            config.rules.align_equals.max_shift_policy,
            MaxAlignShiftPolicy::Split
        );
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
    fn load_walks_up_to_ancestor_directory() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
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
    fn rules_bare_bool_false_leaves_other_knobs_default() {
        let config = Config::from_pyproject_str("[tool.prose.rules]\nalphabetize = false\n")
            .expect("parses");

        assert!(!config.rules.alphabetize.enabled);
        assert!(config.rules.alphabetize.docstring_entries);
    }

    #[test]
    fn rules_inline_table_compiles_regex_knob() {
        let config = Config::from_pyproject_str(
            "[tool.prose.rules]\nsingle-use-variables = { allow-pattern = \"^tmp_\" }\n",
        )
        .expect("parses");

        assert!(config
            .rules
            .single_use_variables
            .allow_pattern
            .is_match("tmp_x"));
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
        assert_eq!(config.rules.align_equals.max_shift.get(), 4);
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
        assert_eq!(config.rules.align_equals.max_shift.get(), 4);
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

        assert!(config
            .rules
            .single_use_variables
            .allow_pattern
            .is_match("tmp_x"));
        assert!(!config
            .rules
            .single_use_variables
            .allow_pattern
            .is_match("xtmp_"));
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

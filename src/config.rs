//! Parses the `[tool.prose]` section from `pyproject.toml`.
//!
//! `Config::load` walks upward from a starting path toward the
//! filesystem root, stopping at the first `pyproject.toml` that
//! carries a `[tool.prose]` section. Reaching the root without a match
//! resolves to full defaults, so Prose works on a fresh Python
//! project with no configuration step. Each rule's configuration
//! lives under `[tool.prose.rules.<name>]`.

use std::num::NonZeroUsize;
use std::path::Path;

use ruff_python_ast::PythonVersion;
use serde::{de::IntoDeserializer, Deserialize};
use thiserror::Error;

pub use crate::rule::RuleConfigs;

/// Configuration shared by the alignment rules (`align_colons`, `align_equals`).
#[derive(Debug, Deserialize)]
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

/// Configuration for the `collection_layout` rule.
#[derive(Debug, Deserialize)]
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

/// The `[tool.prose]` section of a user's `pyproject.toml`.
///
/// `code_line_length` defaults to `Some(88)`. `docstring_line_length`
/// defaults to `Some(76)`. `docstring_structured_policy` defaults
/// to `CodeLineLength`. `target_version` defaults to `None`.
/// Per-rule settings live under `rules`, where each rule's sub-table
/// carries `enabled` plus that rule's own knobs.
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub code_line_length: Option<NonZeroUsize>,
    pub docstring_line_length: Option<NonZeroUsize>,
    pub docstring_structured_policy: DocstringStructuredPolicy,
    pub rules: RuleConfigs,
    pub target_version: Option<PythonVersion>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            code_line_length: NonZeroUsize::new(88),
            docstring_line_length: NonZeroUsize::new(76),
            docstring_structured_policy: DocstringStructuredPolicy::default(),
            rules: RuleConfigs::default(),
            target_version: None,
        }
    }
}

impl Config {
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
        Ok(parse_prose_section(contents, &mut warn_unknown_key)?.unwrap_or_default())
    }

    /// Walks upward from `from` and returns the first `[tool.prose]`
    /// section found in a `pyproject.toml` along the way, or
    /// `Config::default()` if no such section exists on the chain.
    ///
    /// Unknown keys under `[tool.prose]` are logged to stderr as
    /// warnings and ignored, keeping the loader forward-compatible
    /// with rules added in future releases.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Io` if a `pyproject.toml` is found but
    /// cannot be read, and `ConfigError::Toml` if its contents are not
    /// valid TOML.
    pub fn load<P: AsRef<Path>>(from: P) -> Result<Self, ConfigError> {
        Self::load_with_warnings(from, warn_unknown_key)
    }

    /// Shared implementation backing `load`, factored out so tests can
    /// inspect the unknown-key callback without capturing stderr.
    fn load_with_warnings<P, F>(from: P, mut on_unknown: F) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
        F: FnMut(&str),
    {
        from.as_ref()
            .ancestors()
            .find_map(
                |dir| match fs_err::read_to_string(dir.join("pyproject.toml")) {
                    Ok(contents) => parse_prose_section(&contents, &mut on_unknown).transpose(),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => Some(Err(e.into())),
                },
            )
            .unwrap_or_else(|| Ok(Self::default()))
    }
}

/// Failure to load a `[tool.prose]` configuration from a `pyproject.toml`.
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
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DocstringStructuredPolicy {
    #[default]
    CodeLineLength,
    DocstringLineLength,
}

/// What to do when an alignment group's widest padding exceeds the
/// rule's `max-shift`.
///
/// `Split` greedily partitions the group so each contiguous
/// sub-group satisfies the cap, and each sub-group of size `>= 2`
/// aligns independently. `Drop` excludes the widest member(s) from
/// the padding calculation until the cap is satisfied, leaving those
/// members at their original spacing while neighbors align around
/// them. `Skip` leaves the entire group unaligned.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MaxAlignShiftPolicy {
    Drop,
    Skip,
    #[default]
    Split,
}

/// Sub-table shape for rules whose only knob is `enabled`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToggleOnly {
    pub enabled: bool,
}

impl Default for ToggleOnly {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn parse_prose_section<F>(contents: &str, on_unknown: &mut F) -> Result<Option<Config>, ConfigError>
where
    F: FnMut(&str),
{
    let value: toml::Value = toml::from_str(contents)?;
    let Some(prose) = value.get("tool").and_then(|t| t.get("prose")).cloned() else {
        return Ok(None);
    };

    let config: Config = serde_ignored::deserialize(prose.into_deserializer(), |path| {
        on_unknown(&path.to_string())
    })?;

    Ok(Some(config))
}

fn warn_unknown_key(key: &str) {
    eprintln!("warning: unknown key `{key}` in [tool.prose]");
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use tempfile::TempDir;

    use super::*;

    fn assert_toml_error(toml: &str) {
        assert!(matches!(
            Config::from_pyproject_str(toml),
            Err(ConfigError::Toml(_))
        ));
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
    fn from_pyproject_str_with_unknown_key_warns_and_returns_config() {
        let config = Config::from_pyproject_str(
            "[tool.prose]\ncode-line-length = 100\nunknown-future-key = 1\n",
        )
        .expect("parses");

        assert_eq!(config.code_line_length, NonZeroUsize::new(100));
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

        assert!(matches!(result, Err(ConfigError::Toml(_))));
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
                max-shift-policy = "skip"
            "#},
        );

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(
            config.rules.align_colons.max_shift_policy,
            MaxAlignShiftPolicy::Drop
        );
        assert_eq!(
            config.rules.align_equals.max_shift_policy,
            MaxAlignShiftPolicy::Skip
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
    fn load_unknown_key_invokes_callback() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\nunknown-future-key = \"whatever\"\n",
        );

        let mut captured = Vec::new();
        let config = Config::load_with_warnings(tmp.path(), |key| captured.push(key.to_owned()))
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
    fn load_walks_up_to_ancestor_directory() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.code_line_length, NonZeroUsize::new(120));
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
    fn target_version_explicit_value_takes_effect() {
        let config = Config::from_pyproject_str("[tool.prose]\ntarget-version = \"3.14\"\n")
            .expect("parses");

        assert_eq!(config.target_version, Some(PythonVersion::PY314));
    }

    #[test]
    fn target_version_invalid_value_returns_toml_error() {
        assert_toml_error("[tool.prose]\ntarget-version = \"py310\"\n");
    }
}

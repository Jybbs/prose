//! Parses the `[tool.prose]` section from `pyproject.toml`.
//!
//! `Config::load` walks upward from a starting path toward the
//! filesystem root, stopping at the first `pyproject.toml` that
//! carries a `[tool.prose]` section. Reaching the root without a match
//! resolves to full defaults, so *Prose* works on a fresh Python
//! project with no configuration step.

use std::{num::NonZeroUsize, path::Path};

use ruff_python_ast::PythonVersion;
use serde::{de::IntoDeserializer, Deserialize};
use thiserror::Error;

/// The `[tool.prose]` section of a user's `pyproject.toml`.
///
/// `None` on a field means the user did not set that key in their
/// `pyproject.toml`; downstream consumers apply their own fallback
/// rather than reading a synthesized default here.
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub line_length: Option<NonZeroUsize>,
    pub max_align_shift: NonZeroUsize,
    pub max_align_shift_policy: MaxAlignShiftPolicy,
    pub rules: RuleToggles,
    pub target_version: Option<PythonVersion>,
}

impl Config {
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
        Self::load_with_warnings(from, |key| {
            eprintln!("warning: unknown key `{key}` in [tool.prose]");
        })
    }

    /// Shared implementation backing `load`, factored out so tests can
    /// inspect the unknown-key callback without capturing stderr.
    fn load_with_warnings<P, F>(from: P, mut on_unknown: F) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
        F: FnMut(&str),
    {
        for dir in from.as_ref().ancestors() {
            let candidate = dir.join("pyproject.toml");
            match fs_err::read_to_string(&candidate) {
                Ok(contents) => {
                    if let Some(config) = parse_prose_section(&contents, &mut on_unknown)? {
                        return Ok(config);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }

        Ok(Self::default())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_length: None,
            max_align_shift: NonZeroUsize::new(8).expect("8 is non-zero"),
            max_align_shift_policy: MaxAlignShiftPolicy::default(),
            rules: RuleToggles::default(),
            target_version: None,
        }
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

/// What to do when an alignment group's widest padding exceeds
/// `max-align-shift`.
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

/// Per-rule on/off flags parsed from `[tool.prose.rules]`.
///
/// Every flag defaults to `true`. Users opt a rule out by setting it
/// to `false` in their `pyproject.toml`.
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RuleToggles {
    pub align_colons: bool,
    pub align_equals: bool,
    pub align_imports: bool,
    pub alphabetize: bool,
    pub match_case_align: bool,
    pub one_per_line_collections: bool,
    pub singleton_rule: bool,
    pub strip_trailing_commas: bool,
}

impl Default for RuleToggles {
    fn default() -> Self {
        Self {
            align_colons: true,
            align_equals: true,
            align_imports: true,
            alphabetize: true,
            match_case_align: true,
            one_per_line_collections: true,
            singleton_rule: true,
            strip_trailing_commas: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use tempfile::TempDir;

    use super::*;

    fn write_pyproject(dir: &Path, contents: &str) {
        std::fs::write(dir.join("pyproject.toml"), contents).expect("pyproject writes");
    }

    #[test]
    fn load_absent_file_returns_defaults() {
        let tmp = TempDir::new().expect("tempdir");
        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.line_length, None);
        assert_eq!(config.target_version, None);
        assert!(config.rules.align_equals);
    }

    #[test]
    fn load_absent_prose_section_returns_defaults() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[project]\nname = \"x\"\n");

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.line_length, None);
        assert!(config.rules.align_equals);
    }

    #[test]
    fn load_full_override() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            indoc! {r#"
                [tool.prose]
                line-length = 100
                target-version = "3.12"

                [tool.prose.rules]
                align-colons = false
                align-equals = false
                align-imports = false
                alphabetize = false
                match-case-align = false
                one-per-line-collections = false
                singleton-rule = false
                strip-trailing-commas = false
            "#},
        );

        let config = Config::load(tmp.path()).expect("loads");

        assert_eq!(config.line_length, NonZeroUsize::new(100));
        assert_eq!(config.target_version, Some(PythonVersion::PY312));
        assert!(!config.rules.align_colons);
        assert!(!config.rules.strip_trailing_commas);
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
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        let config = Config::load(tmp.path()).expect("loads");

        assert!(!config.rules.align_equals);
        assert!(config.rules.align_colons);
        assert!(config.rules.strip_trailing_commas);
    }

    #[test]
    fn load_picks_nearest_ancestor_when_multiple_configs_exist() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a/b");
        std::fs::create_dir_all(&nested).expect("nested dirs create");

        write_pyproject(tmp.path(), "[tool.prose]\nline-length = 80\n");
        write_pyproject(&nested, "[tool.prose]\nline-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.line_length, NonZeroUsize::new(120));
    }

    #[test]
    fn load_rejects_malformed_target_version() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ntarget-version = \"py310\"\n");

        let result = Config::load(tmp.path());

        assert!(matches!(result, Err(ConfigError::Toml(_))));
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
        assert!(config.rules.align_equals);
    }

    #[test]
    fn load_walks_up_to_ancestor_directory() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&nested).expect("nested dirs create");
        write_pyproject(tmp.path(), "[tool.prose]\nline-length = 120\n");

        let config = Config::load(&nested).expect("loads");

        assert_eq!(config.line_length, NonZeroUsize::new(120));
    }
}

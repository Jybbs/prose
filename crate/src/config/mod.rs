//! Resolves `prose` configuration from `prose.toml`, `.config/prose.toml`,
//! or the `[tool.prose]` table of `pyproject.toml`.
//!
//! `Config::load` walks upward from a starting path toward the
//! filesystem root. In each directory `prose.toml` outranks
//! `.config/prose.toml`, which outranks a `pyproject.toml`, and the
//! nearest directory carrying any of them wins. A `prose.toml` or
//! `.config/prose.toml` holds the config at its document root, whereas a
//! `pyproject.toml` nests it under `[tool.prose]`. Reaching the root
//! without a match resolves to full defaults, so prose works on a
//! fresh project with no configuration step.
//!
//! Each rule's configuration lives under `[tool.prose.rules]`, where
//! a bare bool toggles the rule and a sub-table carries its knobs.
//!
//! `Config::load` yields the base config. Per-file resolution, layering
//! `[[tool.prose.overrides]]` globs and a standalone script's PEP 723
//! block onto that base, lives in [`ConfigSource`].

use std::{collections::HashSet, num::NonZeroUsize, path::Path};

use ruff_python_ast::PythonVersion;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::rule::RuleConfigs;

mod de;
mod load;
mod merge;
mod overrides;
mod schema;
mod script;
mod source;

pub(crate) use de::deserialize_rule;
use de::{deserialize_import_line_length, deserialize_prose};
pub(crate) use load::config_rel_paths;
use load::{ConfigNotice, emit_notice, prose_table_from_str, walk_prose_table};
pub use schema::*;
pub(crate) use source::ConfigSource;

/// The resolved `prose` configuration, read from a `prose.toml` or
/// `.config/prose.toml` document root, or a `pyproject.toml`
/// `[tool.prose]` table.
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
        Self::from_base_table(toml::from_str(contents)?, &mut emit_notice)
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
        match prose_table_from_str(contents)? {
            Some(table) => Self::from_base_table(table, &mut emit_notice),
            None => Ok(Self::default()),
        }
    }

    /// Walks upward from `from`, returning the config from the nearest
    /// directory that carries a `prose.toml`, a `.config/prose.toml`, or
    /// a `pyproject.toml` with a `[tool.prose]` table, or
    /// `Config::default()` if none exists on the chain. Within a directory
    /// `prose.toml` outranks `.config/prose.toml`, which outranks the
    /// `pyproject.toml` table.
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

    /// Deserializes a prose table into a base config, dropping the
    /// `overrides` array that only per-file resolution through
    /// [`ConfigSource`] consults.
    fn from_base_table<F>(mut table: toml::Table, on_notice: &mut F) -> Result<Self, ConfigError>
    where
        F: FnMut(ConfigNotice<'_>),
    {
        table.remove("overrides");
        deserialize_prose(table, on_notice)
    }

    /// Shared implementation backing `load`, factored out so tests can
    /// inspect the emitted notices without capturing stderr.
    fn load_with_notices<P, F>(from: P, mut on_notice: F) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
        F: FnMut(ConfigNotice<'_>),
    {
        match walk_prose_table(from.as_ref(), &mut on_notice)? {
            Some((_, table)) => Self::from_base_table(table, &mut on_notice),
            None => Ok(Self::default()),
        }
    }

    pub(crate) fn allow_set(allow: &[String]) -> HashSet<String> {
        allow.iter().cloned().collect()
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

    pub(crate) fn group_imports_enabled(&self) -> bool {
        self.rules.group_imports.enabled
    }

    /// The budget governing import wrapping, falling back to the code
    /// budget when `import_line_length` is `None`.
    pub(crate) fn import_width(&self) -> usize {
        self.import_line_length
            .map_or_else(|| self.code_width(), NonZeroUsize::get)
    }

    /// The config serialized to TOML, one component of the cache key
    /// for a file it governs.
    pub(crate) fn to_toml(&self) -> String {
        toml::to_string(self).expect("Config serializes")
    }
}

/// Failure to load a `prose` configuration from a config file, a
/// PEP 723 script block, or an override's globs.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Glob(#[from] globset::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

#[cfg(test)]
mod load_tests;
#[cfg(test)]
mod parse_tests;

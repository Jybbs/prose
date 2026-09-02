//! Resolves `prose` configuration from `prose.toml`, `.config/prose.toml`,
//! or the `[tool.prose]` table of `pyproject.toml`.
//!
//! `Config::load` walks upward from a starting path toward the
//! filesystem root. In each directory `prose.toml` outranks
//! `.config/prose.toml`, which outranks a `pyproject.toml`, and the
//! nearest directory carrying any of them wins. A `prose.toml` or
//! `.config/prose.toml` holds the config at its document root, whereas a
//! `pyproject.toml` nests it under `[tool.prose]`. Reaching the root
//! without a match resolves to full defaults, so Prose works on a
//! fresh project with no configuration step.
//!
//! Each rule's configuration lives under `[tool.prose.rules]`, where
//! a bare bool toggles the rule and a sub-table carries its knobs.
//!
//! `Config::load` yields the base config. Per-file resolution, layering
//! `[[tool.prose.overrides]]` globs and a standalone script's PEP 723
//! block onto that base, lives in [`ConfigSource`].
//!
//! The whole tree implements `schemars::JsonSchema`, so `prose
//! schema` prints a JSON Schema carrying every key's type, default,
//! and range.

use std::{num::NonZeroUsize, path::Path};

use ruff_python_ast::PythonVersion;
use rustc_hash::FxHashSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::rules::RuleConfigs;
use crate::{
    primitives::{aligner, comments, fracture, one_row, padding, reserve},
    rules::{
        align_comments::AlignComments, align_equals::AlignEquals, alphabetize_siblings::Reorders,
        normalize_comment_spacing::NormalizeCommentSpacing,
        strip_stranded_padding::StripStrandedPadding,
    },
};

mod de;
mod json_schema;
mod load;
mod merge;
mod overrides;
mod schema;
mod script;
mod source;

pub(crate) use de::deserialize_rule;
use de::{deserialize_optional_cap, deserialize_prose, serialize_optional_cap};
pub(crate) use json_schema::rule_schema;
use load::{ConfigNotice, emit_notice, prose_table_from_str, walk_prose_table};
pub(crate) use load::{NoticeDedup, config_rel_paths, holding_dir};
pub use schema::*;
pub(crate) use source::{ConfigSource, DirSource};

/// The resolved `prose` configuration, read from a `prose.toml` or
/// `.config/prose.toml` document root, or a `pyproject.toml`
/// `[tool.prose]` table.
///
/// `code_line_length` defaults to `Some(88)`. `docstring_line_length`
/// defaults to `Some(76)`. `import_line_length` defaults to `Some(120)`,
/// falling back to `code_line_length` when `false`.
/// `docstring_structured_policy` defaults to `CodeLineLength`.
/// `imports.first_party` defaults to empty. `report_unstable_output`
/// defaults to `true`. `target_version` defaults to `None`. Per-rule
/// settings live under `rules`.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub cache: CacheConfig,
    /// The line budget every length-aware rule honors.
    pub code_line_length: Option<NonZeroUsize>,
    /// The description-prose budget for `wrap-docstrings`.
    pub docstring_line_length: Option<NonZeroUsize>,
    /// The budget structured docstring sections wrap to.
    pub docstring_structured_policy: DocstringStructuredPolicy,
    /// The import-wrap budget for `reflow-imports`, falling back to
    /// `code-line-length` when `false`.
    #[schemars(schema_with = "json_schema::optional_cap_schema")]
    #[serde(
        deserialize_with = "deserialize_optional_cap",
        serialize_with = "serialize_optional_cap"
    )]
    pub import_line_length: Option<NonZeroUsize>,
    pub imports: ImportsConfig,
    /// Reports a rewrite whose settle check names rules as a defect in
    /// Prose, naming the reproducing subset and the invocation that
    /// replays it. `false` lands the rewrite with no notice, governing
    /// the notice surfaces alone, so `check --validate` still runs the
    /// settle check it was passed to run.
    pub report_unstable_output: bool,
    pub rules: RuleConfigs,
    /// The Python runtime the project ships to, read by the
    /// version-gated rules.
    pub target_version: Option<PythonVersion>,
}

impl Config {
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

    /// The config `table` describes, the default where no table was
    /// found.
    fn from_optional_table<F>(
        table: Option<toml::Table>,
        on_notice: &mut F,
    ) -> Result<Self, ConfigError>
    where
        F: FnMut(ConfigNotice<'_>),
    {
        table.map_or_else(
            || Ok(Self::default()),
            |table| Self::from_base_table(table, on_notice),
        )
    }

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
        Self::from_optional_table(prose_table_from_str(contents)?, &mut emit_notice)
    }

    /// Shared implementation backing `load`, factored out so tests can
    /// inspect the emitted notices without capturing stderr.
    fn load_with_notices<P, F>(from: P, mut on_notice: F) -> Result<Self, ConfigError>
    where
        P: AsRef<Path>,
        F: FnMut(ConfigNotice<'_>),
    {
        Self::from_optional_table(
            walk_prose_table(from.as_ref(), &mut on_notice)?.map(|(_, table)| table),
            &mut on_notice,
        )
    }

    /// The alignment settings `config` resolves within `width`, each
    /// line read at the padding and comment rules this config predicts.
    pub(crate) fn align_settings(
        &self,
        config: &AlignmentConfig,
        width: usize,
    ) -> aligner::Settings {
        aligner::Settings::from(config).within(
            width,
            self.stranded_padding(),
            self.comment_settling(),
        )
    }

    pub(crate) fn allow_set(allow: &[String]) -> FxHashSet<String> {
        allow.iter().cloned().collect()
    }

    pub(crate) fn alphabetize_siblings_enabled(&self) -> bool {
        self.rules.alphabetize_siblings.enabled
    }

    pub(crate) fn code_width(&self) -> usize {
        self.code_line_length
            .expect("Config::default synthesizes Some(88)")
            .get()
    }

    /// The two comment rules a measuring rule predicts, so a trailing
    /// comment reads at the gap `align-comments` seats it at and the
    /// opener `normalize-comment-spacing` settles it to.
    fn comment_settling(&self) -> comments::Settling {
        comments::Settling {
            gap: self
                .rules
                .align_comments
                .enabled
                .then_some(AlignComments::SLUG),
            opener: self
                .rules
                .normalize_comment_spacing
                .enabled
                .then_some(NormalizeCommentSpacing::SLUG),
        }
    }

    pub(crate) fn docstring_width(&self) -> usize {
        self.docstring_line_length
            .expect("Config::default synthesizes Some(76)")
            .get()
    }

    /// The `align-equals` reservation a rule measures a construct
    /// against, reserving no column where that rule is off.
    pub(crate) fn equals_reservations(&self) -> reserve::Reservations {
        let settings = self
            .rules
            .align_equals
            .enabled
            .then(|| self.equals_settings());
        reserve::Reservations::new(AlignEquals::SLUG, settings, self.one_row_settings())
    }

    /// The alignment settings `align-equals` runs under, resolving
    /// within the code width and releasing a group's head, since its
    /// rows reach their settled width under it.
    pub(crate) fn equals_settings(&self) -> aligner::Settings {
        self.align_settings(&self.rules.align_equals, self.code_width())
            .releasing_heads()
    }

    pub(crate) fn first_party(&self) -> Vec<String> {
        self.imports.first_party.clone()
    }

    /// The terms a fractured argument list closes under, closing
    /// none where `reflow-calls` is off.
    pub(crate) fn fracture_settings(&self) -> fracture::Settings<'static> {
        fracture::Settings::from(&self.rules.reflow_calls)
    }

    pub(crate) fn group_imports_enabled(&self) -> bool {
        self.rules.group_imports.enabled
    }

    /// The alignment settings `align-imports` runs under, resolving
    /// within the import width, read by the rule itself and by the
    /// forecast `reflow-imports` packs against, so the column the
    /// forecast names is one the capped run seats.
    pub(crate) fn import_align_settings(&self) -> aligner::Settings {
        self.align_settings(&self.rules.align_imports, self.import_width())
    }

    /// The budget governing import wrapping, falling back to the code
    /// budget when `import_line_length` is `None`.
    pub(crate) fn import_width(&self) -> usize {
        self.import_line_length
            .map_or_else(|| self.code_width(), NonZeroUsize::get)
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

    /// Loads the base config for `from`, routing its notices through a
    /// run-scoped `dedup` so a run that reloads the same config per file
    /// warns each key once across both loads.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Io` if a config file is found but cannot be
    /// read, and `ConfigError::Toml` if its contents are not valid TOML.
    pub(crate) fn load_deduped<P: AsRef<Path>>(
        from: P,
        dedup: &NoticeDedup,
    ) -> Result<Self, ConfigError> {
        Self::load_with_notices(from, |notice| dedup.emit(notice))
    }

    /// The terms a construct reaches one row under, read by every rule
    /// deciding where that construct lands.
    pub(crate) fn one_row_settings(&self) -> one_row::Settings<'static> {
        one_row::Settings::from(self)
    }

    /// The leaf sorts a measuring rule forecasts, so an entry reads with
    /// the separator `alphabetize-siblings` leaves after it.
    pub(crate) fn reorders(&self) -> Reorders {
        Reorders::from_config(self)
    }

    /// The padding rule a measuring rule predicts, so a row reads at the
    /// width `strip-stranded-padding` settles it to.
    pub(crate) fn stranded_padding(&self) -> padding::Stranding {
        padding::Stranding::new(
            StripStrandedPadding::SLUG,
            self.rules.strip_stranded_padding.enabled,
        )
    }

    /// The keys this config sets away from the default, serialized to
    /// TOML. Empty for a config running on the defaults.
    pub(crate) fn to_changed_toml(&self) -> String {
        let mut set = toml::Table::try_from(self).expect("Config serializes");
        let defaults = toml::Table::try_from(Self::default()).expect("Config serializes");
        merge::without_defaults(&mut set, &defaults);
        toml::to_string(&set).expect("Config serializes")
    }

    /// The config serialized to TOML.
    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("Config serializes")
    }
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
            report_unstable_output: true,
            rules: RuleConfigs::default(),
            target_version: None,
        }
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
mod tests;

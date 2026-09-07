//! Finds config files on disk, climbing from a starting path to the
//! nearest directory that holds a prose table. That table becomes the
//! `Config` a run reads.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::{Deserialize, de::IntoDeserializer};

use super::{
    Config, ConfigError,
    notice::{ConfigForm, ConfigNotice, PRECEDENCE, unknown_keys},
    sink::{NoticeDedup, emit_notice},
};

impl Config {
    /// Builds the config `table` describes, or the default where there
    /// is no table.
    fn from_optional_table<F>(
        table: Option<toml::Table>,
        on_notice: &mut F,
    ) -> Result<Self, ConfigError>
    where
        F: FnMut(ConfigNotice<'_>),
    {
        table.map_or_else(
            || Ok(Self::default()),
            |table| Self::from_base_table(table, &mut unknown_keys(on_notice)),
        )
    }

    /// Parses a `pyproject.toml` snippet directly from a string.
    ///
    /// Returns `Config::default()` when `contents` carries no
    /// `[tool.prose]` section. Each unknown key under `[tool.prose]`
    /// produces a notice on stderr.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Toml` when `contents` is not valid TOML.
    pub fn from_pyproject_str(contents: &str) -> Result<Self, ConfigError> {
        Self::from_optional_table(prose_table_from_str(contents)?, &mut emit_notice)
    }

    /// Walks upward from `from`, returning the config from the nearest
    /// directory that carries a `prose.toml`, a `.config/prose.toml`, or
    /// a `pyproject.toml` with a `[tool.prose]` table, or
    /// `Config::default()` if none exists on the chain. Within a directory
    /// `prose.toml` outranks `.config/prose.toml`, which outranks the
    /// `pyproject.toml` table.
    ///
    /// Unknown keys and the precedence outcome are logged to stderr and
    /// ignored.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Io` if a config file is found but cannot be
    /// read, and `ConfigError::Toml` if its contents are not valid TOML.
    pub fn load<P: AsRef<Path>>(from: P) -> Result<Self, ConfigError> {
        Self::load_with_notices(from.as_ref(), emit_notice)
    }

    /// Loads the base config for `from`, routing its notices through a
    /// run-scoped `dedup`, so a run reloading the same config per file
    /// emits one notice per key across both loads.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Io` if a config file is found but cannot be
    /// read, and `ConfigError::Toml` if its contents are not valid TOML.
    pub(crate) fn load_deduped<P: AsRef<Path>>(
        from: P,
        dedup: &NoticeDedup,
    ) -> Result<Self, ConfigError> {
        Self::load_with_notices(from.as_ref(), |notice| dedup.emit(notice))
    }

    /// Loads the config governing `from`, routing each notice through
    /// `on_notice`.
    pub(super) fn load_with_notices<F>(from: &Path, mut on_notice: F) -> Result<Self, ConfigError>
    where
        F: FnMut(ConfigNotice<'_>),
    {
        Self::from_optional_table(
            walk_prose_table(from, &mut on_notice)?.map(|(_, table)| table),
            &mut on_notice,
        )
    }
}

/// Lists each config form's path relative to its directory, the set the
/// server's file watcher registers against.
pub(crate) fn config_rel_paths() -> [&'static str; PRECEDENCE.len()] {
    PRECEDENCE.map(ConfigForm::rel_path)
}

/// Returns the directory holding `file`, or `file` itself at a root.
pub(crate) fn holding_dir(file: &Path) -> &Path {
    file.parent().unwrap_or(file)
}

/// Extracts the `[tool.prose]` table from a TOML document, or `None`
/// where the document has no `tool.prose` entry. The `pyproject.toml`
/// read and the PEP 723 script block both extract their table here.
///
/// # Errors
///
/// Returns `ConfigError::Toml` when `contents` is not valid TOML or its
/// `tool.prose` is present but not a table.
pub(super) fn prose_table_from_str(contents: &str) -> Result<Option<toml::Table>, ConfigError> {
    let value: toml::Value = toml::from_str(contents)?;
    match prose_value(&value) {
        Some(prose) => Ok(Some(toml::Table::deserialize(
            prose.clone().into_deserializer(),
        )?)),
        None => Ok(None),
    }
}

/// Walks upward from `from`, returning the directory and prose table of
/// the nearest directory carrying a recognized config form, or `None`
/// when the chain to the root carries none. Within a directory the order
/// is `prose.toml`, then `.config/prose.toml`, then a `pyproject.toml`
/// `[tool.prose]` table, and the walk emits a precedence notice for each
/// lower form present alongside the winner.
///
/// # Errors
///
/// Returns `ConfigError::Io` if a config file is found but cannot be
/// read, and `ConfigError::Toml` if its contents are not valid TOML.
pub(super) fn walk_prose_table<F>(
    from: &Path,
    on_notice: &mut F,
) -> Result<Option<(PathBuf, toml::Table)>, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    for dir in from.ancestors() {
        let mut resolved: Option<(ConfigForm, toml::Table)> = None;
        for form in PRECEDENCE {
            let Some(table) = read_form(form, dir)? else {
                continue;
            };
            if let Some((winner, _)) = &resolved {
                on_notice(ConfigNotice::Precedence {
                    dir,
                    shadowed: form,
                    winner: *winner,
                });
            } else {
                resolved = Some((form, table));
            }
        }
        if let Some((_, table)) = resolved {
            return Ok(Some((dir.to_path_buf(), table)));
        }
    }
    Ok(None)
}

fn prose_value(value: &toml::Value) -> Option<&toml::Value> {
    value.get("tool").and_then(|tool| tool.get("prose"))
}

/// Reads `form`'s prose table from `dir`, yielding `None` when the file
/// is missing or when a `pyproject.toml` has no `[tool.prose]`.
fn read_form(form: ConfigForm, dir: &Path) -> Result<Option<toml::Table>, ConfigError> {
    let Some(contents) = read_optional(dir.join(form.rel_path()))? else {
        return Ok(None);
    };
    match form {
        ConfigForm::DotConfigProseToml | ConfigForm::ProseToml => {
            Ok(Some(toml::from_str(&contents)?))
        }
        ConfigForm::PyprojectTable => prose_table_from_str(&contents),
    }
}

/// Reads a config file that may not exist. `NotADirectory` counts as
/// missing too, which is what a walk starting at a file reports.
fn read_optional(path: PathBuf) -> Result<Option<String>, ConfigError> {
    match fs_err::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) if matches!(e.kind(), ErrorKind::NotADirectory | ErrorKind::NotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

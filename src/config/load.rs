//! Config-file discovery: the upward walk yielding the nearest prose
//! table, the TOML reads, and the precedence / unknown-key notices.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::{Deserialize, de::IntoDeserializer};

use super::{ConfigError, PROSE_TOML, PYPROJECT_TOML};

/// A diagnostic surfaced while resolving configuration.
pub(super) enum ConfigNotice<'a> {
    /// A `prose.toml` outranked a `[tool.prose]` table in a
    /// `pyproject.toml` sharing its directory. Carries that directory.
    ProseTomlPrecedence(&'a Path),
    /// An unrecognized key under the prose table. Carries the dotted
    /// key path.
    UnknownKey(&'a str),
}

pub(super) fn emit_notice(notice: ConfigNotice<'_>) {
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

/// Extracts the `[tool.prose]` table from a TOML document, shared by the
/// `pyproject.toml` read and the PEP 723 script block. `None` when the
/// document carries no `tool.prose` entry.
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

/// Reads a config file that may not exist. `NotADirectory` reads as
/// absent too, so a walk whose starting path is itself a file skips
/// the join through that file rather than erroring.
fn read_optional(path: PathBuf) -> Result<Option<String>, ConfigError> {
    match fs_err::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) if matches!(e.kind(), ErrorKind::NotADirectory | ErrorKind::NotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Walks upward from `from`, returning the directory and prose table of
/// the nearest `prose.toml` or `pyproject.toml` `[tool.prose]`, or `None`
/// when the chain to the root carries neither. A `prose.toml` outranks a
/// same-directory `pyproject.toml` and stops the walk.
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
        if let Some(contents) = read_optional(dir.join(PROSE_TOML))? {
            if pyproject_declares_prose(dir) {
                on_notice(ConfigNotice::ProseTomlPrecedence(dir));
            }
            return Ok(Some((dir.to_path_buf(), toml::from_str(&contents)?)));
        }
        if let Some(contents) = read_optional(dir.join(PYPROJECT_TOML))?
            && let Some(table) = prose_table_from_str(&contents)?
        {
            return Ok(Some((dir.to_path_buf(), table)));
        }
    }
    Ok(None)
}

fn prose_value(value: &toml::Value) -> Option<&toml::Value> {
    value.get("tool").and_then(|tool| tool.get("prose"))
}

fn pyproject_declares_prose(dir: &Path) -> bool {
    read_optional(dir.join(PYPROJECT_TOML))
        .ok()
        .flatten()
        .and_then(|contents| prose_table_from_str(&contents).ok().flatten())
        .is_some()
}

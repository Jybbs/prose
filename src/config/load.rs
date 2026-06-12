//! Config-file discovery: the upward walk, TOML reads, and the
//! precedence / unknown-key notices.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use super::de::deserialize_prose;
use super::{Config, ConfigError, PYPROJECT_TOML};
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

pub(super) fn parse_prose_toml<F>(contents: &str, on_notice: &mut F) -> Result<Config, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    deserialize_prose(toml::from_str(contents)?, on_notice)
}

pub(super) fn parse_pyproject<F>(
    contents: &str,
    on_notice: &mut F,
) -> Result<Option<Config>, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    let value: toml::Value = toml::from_str(contents)?;
    let Some(prose) = prose_table(&value).cloned() else {
        return Ok(None);
    };
    Ok(Some(deserialize_prose(prose, on_notice)?))
}

pub(super) fn pyproject_declares_prose(dir: &Path) -> bool {
    fs_err::read_to_string(dir.join(PYPROJECT_TOML))
        .ok()
        .and_then(|contents| toml::from_str::<toml::Value>(&contents).ok())
        .is_some_and(|value| prose_table(&value).is_some())
}

/// Reads a config file that may not exist. `NotADirectory` reads as
/// absent too, so a walk whose starting path is itself a file skips
/// the join through that file rather than erroring.
pub(super) fn read_optional(path: PathBuf) -> Result<Option<String>, ConfigError> {
    match fs_err::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) if matches!(e.kind(), ErrorKind::NotADirectory | ErrorKind::NotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn prose_table(value: &toml::Value) -> Option<&toml::Value> {
    value.get("tool").and_then(|tool| tool.get("prose"))
}

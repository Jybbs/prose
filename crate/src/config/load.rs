//! Config-file discovery: the upward walk yielding the nearest prose
//! table, the TOML reads, and the precedence / unknown-key notices.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::{Deserialize, de::IntoDeserializer};

use super::ConfigError;

/// A recognized prose-config source within a directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConfigForm {
    DotConfigProseToml,
    ProseToml,
    PyprojectTable,
}

impl ConfigForm {
    /// The forms in the precedence order the walk applies, highest first.
    const PRECEDENCE: [Self; 3] = [
        Self::ProseToml,
        Self::DotConfigProseToml,
        Self::PyprojectTable,
    ];

    /// How this form names itself in a precedence notice.
    fn label(self) -> &'static str {
        match self {
            Self::DotConfigProseToml | Self::ProseToml => self.rel_path(),
            Self::PyprojectTable => "the [tool.prose] table",
        }
    }

    /// Reads this form's prose table from `dir`, yielding `None` when the
    /// file is absent or a `pyproject.toml` carries no `[tool.prose]`.
    fn read(self, dir: &Path) -> Result<Option<toml::Table>, ConfigError> {
        let Some(contents) = read_optional(dir.join(self.rel_path()))? else {
            return Ok(None);
        };
        match self {
            Self::DotConfigProseToml | Self::ProseToml => Ok(Some(toml::from_str(&contents)?)),
            Self::PyprojectTable => prose_table_from_str(&contents),
        }
    }

    /// This form's directory-relative path.
    fn rel_path(self) -> &'static str {
        match self {
            Self::DotConfigProseToml => ".config/prose.toml",
            Self::ProseToml => "prose.toml",
            Self::PyprojectTable => "pyproject.toml",
        }
    }
}

/// A diagnostic surfaced while resolving configuration.
pub(super) enum ConfigNotice<'a> {
    /// A higher-precedence config form shadowed a lower one present in
    /// the same directory. Carries that directory and the two forms.
    Precedence {
        dir: &'a Path,
        shadowed: ConfigForm,
        winner: ConfigForm,
    },
    /// An unrecognized key under the prose table. Carries the dotted
    /// key path.
    UnknownKey(&'a str),
}

/// The directory-relative path of every recognized config form, the
/// set the server's file watcher registers against.
pub(crate) fn config_rel_paths() -> [&'static str; ConfigForm::PRECEDENCE.len()] {
    ConfigForm::PRECEDENCE.map(ConfigForm::rel_path)
}

pub(super) fn emit_notice(notice: ConfigNotice<'_>) {
    match notice {
        ConfigNotice::Precedence {
            dir,
            shadowed,
            winner,
        } => eprintln!(
            "note: {} takes precedence over {} in {}",
            winner.label(),
            shadowed.label(),
            dir.display(),
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
/// the nearest directory carrying a recognized config form, or `None`
/// when the chain to the root carries none. Within a directory the order
/// is `prose.toml`, then `.config/prose.toml`, then a `pyproject.toml`
/// `[tool.prose]` table, and each lower form present alongside the winner
/// raises a precedence notice.
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
        for form in ConfigForm::PRECEDENCE {
            let Some(table) = form.read(dir)? else {
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

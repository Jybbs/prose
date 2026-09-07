//! The three config file forms, and the notices raised while resolving
//! them. One `Display` renders every notice, so each sink prints the
//! same line.

use std::{fmt, path::Path};

use super::de::unknown_key_notice;

/// A recognized prose-config source within a directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConfigForm {
    DotConfigProseToml,
    ProseToml,
    PyprojectTable,
}

impl ConfigForm {
    /// Renders this form's name for a precedence notice.
    fn label(self) -> &'static str {
        match self {
            Self::DotConfigProseToml | Self::ProseToml => self.rel_path(),
            Self::PyprojectTable => "the [tool.prose] table",
        }
    }

    /// Returns this form's directory-relative path.
    pub(super) fn rel_path(self) -> &'static str {
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

impl fmt::Display for ConfigNotice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Precedence {
                dir,
                shadowed,
                winner,
            } => write!(
                f,
                "note: {} takes precedence over {} in {}",
                winner.label(),
                shadowed.label(),
                dir.display(),
            ),
            Self::UnknownKey(key) => f.write_str(&unknown_key_notice(key)),
        }
    }
}

/// Wraps a notice sink in the unknown-key callback the prose
/// deserialization takes.
pub(super) fn unknown_keys<'a, F>(on_notice: &'a mut F) -> impl FnMut(&str) + 'a
where
    F: FnMut(ConfigNotice<'_>),
{
    |key| on_notice(ConfigNotice::UnknownKey(key))
}

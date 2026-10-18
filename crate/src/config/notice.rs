//! Names the config file forms and the notices raised while resolving
//! them, rendering every notice through one `Display` so each sink
//! prints the same line.

use std::{fmt, path::Path};

use super::de::TableNotice;

/// The config forms, highest precedence first.
pub(super) const PRECEDENCE: [ConfigForm; 3] = [
    ConfigForm::ProseToml,
    ConfigForm::DotConfigProseToml,
    ConfigForm::PyprojectTable,
];

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
    /// A notice the prose table raised while it deserialized.
    Table(TableNotice<'a>),
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
            Self::Table(notice) => notice.fmt(f),
        }
    }
}

/// Wraps a notice sink in the table-notice callback the prose
/// deserialization takes.
pub(super) fn table_notices<'a, F>(on_notice: &'a mut F) -> impl FnMut(TableNotice<'_>) + 'a
where
    F: FnMut(ConfigNotice<'_>),
{
    |notice| on_notice(ConfigNotice::Table(notice))
}

//! Exit-code matrix. Higher discriminants shadow lower ones via `Ord::max`.

use std::{fmt::Write, process::ExitCode};

use crate::diagnostics::Severity;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub(crate) enum ExitStatus {
    #[default]
    Clean = 0,
    FormatChange = 1,
    LintViolation = 2,
    ParseError = 3,
    ConfigError = 4,
}

impl ExitStatus {
    /// Every status, ascending by the code it exits with.
    pub(crate) const ALL: [Self; 5] = [
        Self::Clean,
        Self::FormatChange,
        Self::LintViolation,
        Self::ParseError,
        Self::ConfigError,
    ];

    /// The `prose --help` matrix, one row per status.
    pub(crate) fn matrix() -> String {
        Self::ALL
            .into_iter()
            .fold("Exit codes:".to_owned(), |mut rows, status| {
                write!(rows, "\n  {}    {}", status as u8, status.describe())
                    .expect("writes to a string");
                rows
            })
    }

    /// The line the help matrix prints for this status.
    fn describe(self) -> &'static str {
        match self {
            Self::Clean => "Clean: no diagnostics, no rewrites pending",
            Self::FormatChange => "Format would change: at least one Severity::Format diagnostic",
            Self::LintViolation => "Lint violation: at least one Severity::Lint diagnostic",
            Self::ParseError => "Parse error: input could not be parsed as Python",
            Self::ConfigError => {
                "Config error: pyproject.toml, --select / --ignore, or arg validation"
            }
        }
    }
}

impl From<Severity> for ExitStatus {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Format => Self::FormatChange,
            Severity::Lint => Self::LintViolation,
        }
    }
}

impl From<ExitStatus> for ExitCode {
    fn from(s: ExitStatus) -> Self {
        ExitCode::from(s as u8)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn default_is_clean() {
        assert_eq!(ExitStatus::default(), ExitStatus::Clean);
    }

    #[rstest]
    #[case(0, ExitStatus::Clean)]
    #[case(1, ExitStatus::FormatChange)]
    #[case(2, ExitStatus::LintViolation)]
    #[case(3, ExitStatus::ParseError)]
    #[case(4, ExitStatus::ConfigError)]
    fn discriminant_matches_matrix(#[case] expected: u8, #[case] status: ExitStatus) {
        assert_eq!(status as u8, expected);
    }

    #[test]
    fn from_severity_format_is_format_change() {
        assert_eq!(ExitStatus::from(Severity::Format), ExitStatus::FormatChange);
    }

    #[test]
    fn from_severity_lint_is_lint_violation() {
        assert_eq!(ExitStatus::from(Severity::Lint), ExitStatus::LintViolation);
    }

    #[test]
    fn ord_matches_matrix() {
        assert!(ExitStatus::ALL.is_sorted());
    }

    #[test]
    fn matrix_lists_every_status_against_its_code() {
        let matrix = ExitStatus::matrix();
        let rows: Vec<&str> = matrix.lines().skip(1).collect();

        assert_eq!(rows.len(), ExitStatus::ALL.len());
        for (status, row) in ExitStatus::ALL.into_iter().zip(rows) {
            assert_eq!(row, format!("  {}    {}", status as u8, status.describe()));
        }
    }
}

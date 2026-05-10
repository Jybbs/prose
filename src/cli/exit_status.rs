//! Exit-code matrix. Higher discriminants shadow lower ones via `Ord::max`.

use std::process::ExitCode;

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
    use super::*;

    const ASCENDING: [ExitStatus; 5] = [
        ExitStatus::Clean,
        ExitStatus::FormatChange,
        ExitStatus::LintViolation,
        ExitStatus::ParseError,
        ExitStatus::ConfigError,
    ];

    #[test]
    fn default_is_clean() {
        assert_eq!(ExitStatus::default(), ExitStatus::Clean);
    }

    #[test]
    fn discriminants_match_matrix() {
        for (i, status) in ASCENDING.iter().enumerate() {
            assert_eq!(*status as u8, i as u8);
        }
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
    fn into_exit_code_compiles_for_each_variant() {
        for status in ASCENDING {
            let _: ExitCode = status.into();
        }
    }

    #[test]
    fn ord_matches_matrix() {
        assert!(ASCENDING.is_sorted());
    }
}

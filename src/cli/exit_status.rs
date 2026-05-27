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
    use pretty_assertions::assert_eq;
    use rstest::rstest;

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

    #[rstest]
    #[case(ExitStatus::Clean)]
    #[case(ExitStatus::ConfigError)]
    #[case(ExitStatus::FormatChange)]
    #[case(ExitStatus::LintViolation)]
    #[case(ExitStatus::ParseError)]
    fn into_exit_code_compiles_for_each_variant(#[case] status: ExitStatus) {
        let _: ExitCode = status.into();
    }

    #[test]
    fn ord_matches_matrix() {
        assert!(ASCENDING.is_sorted());
    }
}

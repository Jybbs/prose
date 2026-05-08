//! `Diagnostic` and `Severity` definitions.

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use crate::rule::RuleId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub fix: Option<Edit>,
    pub message: String,
    pub range: TextRange,
    pub rule: RuleId,
    pub severity: Severity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Format,
    Lint,
}

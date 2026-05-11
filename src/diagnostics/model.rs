//! `Diagnostic` and `Severity` definitions.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};

use crate::rule::RuleId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub fix: Option<Edit>,
    pub message: String,
    pub range: TextRange,
    pub rule: RuleId,
    pub severity: Severity,
}

impl Diagnostic {
    /// Builds a `Severity::Format` diagnostic that carries `edit` as its
    /// proposed fix and inherits the edit's range.
    pub fn format(rule: RuleId, edit: Edit, message: String) -> Self {
        let range = edit.range();
        Self {
            fix: Some(edit),
            message,
            range,
            rule,
            severity: Severity::Format,
        }
    }

    /// Builds a `Severity::Lint` diagnostic with no associated fix.
    pub fn lint(rule: RuleId, range: TextRange, message: String) -> Self {
        Self {
            fix: None,
            message,
            range,
            rule,
            severity: Severity::Lint,
        }
    }
}

impl Ranged for Diagnostic {
    fn range(&self) -> TextRange {
        self.range
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Format,
    Lint,
}

//! `Diagnostic` and `Severity` definitions.

use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{Ranged, TextRange};
use serde::{Deserialize, Serialize};

use crate::rule::RuleId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub fix: Option<Fix>,
    pub message: String,
    pub range: TextRange,
    pub rule: RuleId,
    pub severity: Severity,
}

impl Diagnostic {
    /// Builds a `Severity::Format` diagnostic carrying `edits` as one
    /// `Applicability::Safe` fix, its range covering every edit in the
    /// group. `edits` must be non-empty.
    pub fn format(rule: RuleId, edits: Vec<Edit>, message: String) -> Self {
        let mut edits = edits.into_iter();
        let first = edits
            .next()
            .expect("a format diagnostic carries at least one edit");
        let fix = Fix::safe_edits(first, edits);
        let range = fix
            .edits()
            .iter()
            .map(Ranged::range)
            .reduce(TextRange::cover)
            .expect("a format diagnostic carries at least one edit");
        Self {
            fix: Some(fix),
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

    /// Builds a `Severity::Lint` diagnostic carrying `edit` as an
    /// `Applicability::DisplayOnly` fix, recorded for display but never
    /// applied by the pipeline.
    pub fn suggestion(rule: RuleId, range: TextRange, message: String, edit: Edit) -> Self {
        Self {
            fix: Some(Fix::display_only_edit(edit)),
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Severity {
    Format,
    Lint,
}

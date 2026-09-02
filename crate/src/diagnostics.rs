//! `Diagnostic` and `Severity` definitions.

use std::collections::BTreeSet;

use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{Ranged, TextRange};
use serde::{Deserialize, Serialize};

use crate::rules::RuleId;

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

impl Severity {
    pub(crate) fn is_format(self) -> bool {
        matches!(self, Self::Format)
    }

    pub(crate) fn is_lint(self) -> bool {
        matches!(self, Self::Lint)
    }
}

/// The rules whose fix groups survived a run, read off the
/// `Severity::Format` diagnostics it emitted.
pub(crate) fn fired_rules(diagnostics: &[Diagnostic]) -> BTreeSet<RuleId> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity.is_format())
        .map(|diagnostic| diagnostic.rule)
        .collect()
}

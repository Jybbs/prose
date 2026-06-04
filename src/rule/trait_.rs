//! The `Rule` trait every concrete rule implements.

use ruff_diagnostics::Edit;

use super::id::RuleId;
use super::registry::message_for_id;
use crate::{diagnostics::Diagnostic, source::Source};

/// Every rule in Prose implements this trait and nothing more.
///
/// Implementations inspect `source` and return the edits that would
/// bring it into conformance, partitioned into fix groups, the
/// `Severity::Lint` diagnostics they surface without an edit, or both.
/// An empty outer `Vec` from `apply` skips the reparse for that rule.
///
/// Rules must be `Send + Sync` so that the pipeline can run across
/// files in parallel without moving the rule list per worker.
pub(crate) trait Rule: Send + Sync {
    /// Computes the edits this rule would apply to `source`,
    /// partitioned into fix groups. Each inner `Vec` is one fix that
    /// the pipeline maps to a single diagnostic, and the edits across
    /// all groups must not overlap after sorting. The pipeline's
    /// applicator declines an overlapping group rather than splicing it.
    fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
        Vec::new()
    }

    /// Stable, kebab-case identifier matching the rule's
    /// `[tool.prose.rules]` key. Surfaces in `--select`,
    /// `# prose: ignore`, and diagnostic output.
    fn id(&self) -> RuleId;

    /// Lint-only side channel emitting `Severity::Lint` diagnostics
    /// the pipeline cannot derive from an edit. The default returns
    /// no diagnostics, so auto-fix rules need not override.
    fn lint(&self, _source: &Source) -> Vec<Diagnostic> {
        Vec::new()
    }

    /// One-line imperative carried as `Diagnostic.message`. Defaults
    /// to the registry-supplied string for `self.id()`.
    fn message(&self) -> &'static str {
        message_for_id(self.id())
    }
}

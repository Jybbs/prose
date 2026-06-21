//! Suppression filtering: drops fix-group edits and lint
//! diagnostics that fall under a `# prose: ignore` directive or a
//! suppressed span.

use ruff_diagnostics::Edit;
use ruff_text_size::Ranged;

use crate::{
    diagnostics::{Diagnostic, Severity},
    rule::{Rule, RuleId},
    source::Source,
    suppression::SuppressionMap,
};
/// Drops the lint diagnostics a `# prose: ignore[<id>]` directive
/// covers, matched per line and rule.
pub(super) fn drop_suppressed_lints(
    diagnostics: &mut Vec<Diagnostic>,
    source: &Source,
    suppression: &SuppressionMap,
) {
    if suppression.has_lint_suppression() {
        diagnostics.retain(|d| {
            d.severity != Severity::Lint
                || !suppression.is_lint_suppressed_at(source.line_index(d.start()), d.rule)
        });
    }
}

/// Applies `rule` to `source` and returns its fix groups with the
/// suppressed edits and the groups they emptied removed.
pub(super) fn prepared_groups(
    rule: &dyn Rule,
    source: &Source,
    suppression: &SuppressionMap,
    rule_id: RuleId,
) -> Vec<Vec<Edit>> {
    let mut groups = rule.apply(source);
    retain_unsuppressed(&mut groups, source, suppression, rule_id);
    groups.retain(|group| !group.is_empty());
    groups
}

/// Yields `rule`'s lint diagnostics, dropping the ones whose range
/// falls within a format-suppressed span.
pub(super) fn unsuppressed_lints<'a>(
    rule: &dyn Rule,
    source: &Source,
    suppression: &'a SuppressionMap,
) -> impl Iterator<Item = Diagnostic> + 'a {
    rule.lint(source)
        .into_iter()
        .filter(move |d| !suppression.intersects(d.range))
}

/// Drops the edits a format-suppression directive covers from each
/// group, per rule. A `# fmt: off` span or `# prose: skip[<id>]`
/// removes the edits it overlaps, leaving the rest of the group intact.
fn retain_unsuppressed(
    groups: &mut [Vec<Edit>],
    source: &Source,
    suppression: &SuppressionMap,
    rule: RuleId,
) {
    if suppression.has_format_suppression() || suppression.has_skip_suppression() {
        for group in groups.iter_mut() {
            group.retain(|edit| {
                !suppression.intersects(edit)
                    && !suppression.is_format_suppressed_at(source.line_index(edit.start()), rule)
            });
        }
    }
}

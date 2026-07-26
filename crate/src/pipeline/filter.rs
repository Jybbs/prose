//! Suppression filtering: drops the fix groups and lint diagnostics
//! that fall under a `# prose: ignore` directive or a suppressed span.

use ruff_diagnostics::Edit;
use ruff_text_size::Ranged;

use crate::{
    diagnostics::Diagnostic,
    rule::{Rule, RuleId},
    source::Source,
};

/// Drops the lint diagnostics a `# prose: ignore[<id>]` directive
/// covers, matched per line and rule.
pub(super) fn drop_suppressed_lints(diagnostics: &mut Vec<Diagnostic>, source: &Source) {
    let suppression = source.suppression_map();
    if suppression.has_lint_suppression() {
        diagnostics.retain(|d| {
            !d.severity.is_lint()
                || !suppression.is_lint_suppressed_at(source.line_index(d.start()), d.rule)
        });
    }
}

/// Applies `rule` to `source` and returns its fix groups with the
/// suppressed and empty ones removed. A group is dropped whole as soon
/// as one of its edits falls under a `# fmt: off` span or a
/// `# prose: skip[<id>]` directive, so a rule's co-dependent edits
/// never split across a suppression boundary.
pub(super) fn prepared_groups(rule: &dyn Rule, source: &Source, rule_id: RuleId) -> Vec<Vec<Edit>> {
    let mut groups = rule.apply(source);
    let suppression = source.suppression_map();
    groups.retain(|g| !g.is_empty() && !g.iter().any(|e| suppression.suppresses(e, rule_id)));
    groups
}

/// Yields `rule`'s lint diagnostics, dropping the ones whose range
/// falls within a format-suppressed span.
pub(super) fn unsuppressed_lints<'a>(
    rule: &dyn Rule,
    source: &'a Source,
) -> impl Iterator<Item = Diagnostic> + 'a {
    let suppression = source.suppression_map();
    rule.lint(source)
        .into_iter()
        .filter(move |d| !suppression.intersects(d.range))
}

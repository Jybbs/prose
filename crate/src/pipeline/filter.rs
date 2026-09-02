//! Suppression filtering: drops the fix groups and lint diagnostics
//! that fall under a `# prose: ignore` directive or a suppressed span.

use ruff_diagnostics::Edit;
use ruff_text_size::Ranged;

use crate::{diagnostics::Diagnostic, rule::Rule, source::Source};

/// Applies `rule` to `source` and returns its fix groups with the
/// suppressed and empty ones removed. A group is dropped whole as soon
/// as one of its edits falls under a suppression span for `rule`.
pub(super) fn prepared_groups(rule: &dyn Rule, source: &Source) -> Vec<Vec<Edit>> {
    let mut groups = rule.apply(source);
    let rule_id = rule.id();
    let suppression = source.suppression_map();
    groups.retain(|g| !g.is_empty() && !g.iter().any(|e| suppression.suppresses(e, rule_id)));
    groups
}

/// Lints `source` through every rule in `rules`, dropping the
/// diagnostics a format-suppressed span covers and the ones a
/// `# prose: ignore[<id>]` directive names on their own line.
pub(super) fn settled_lints(rules: &[Box<dyn Rule>], source: &Source) -> Vec<Diagnostic> {
    let suppression = source.suppression_map();
    let lints = suppression.has_lint_suppression();
    rules
        .iter()
        .flat_map(|rule| rule.lint(source))
        .filter(|d| !suppression.intersects(d.range))
        .filter(|d| {
            !lints || !suppression.is_lint_suppressed_at(source.line_index(d.start()), d.rule)
        })
        .collect()
}

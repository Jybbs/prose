//! Suppression filtering: drops the fix groups and lint diagnostics
//! that fall under a `# prose: ignore` directive or a suppressed span,
//! and hands the diagnostics each `ignore` silenced to the rules'
//! audits.

use ruff_diagnostics::Edit;
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use super::held::Held;
use crate::{diagnostics::Diagnostic, rules::Rule, source::Source};

/// Applies `rule` to `source` and returns its fix groups with the
/// suppressed and empty ones removed. A group is dropped whole as soon
/// as one of its edits falls under a suppression span for `rule`.
pub(crate) fn prepared_groups(rule: &dyn Rule, source: &Source) -> Vec<Vec<Edit>> {
    let mut groups = rule.apply(source);
    let rule_id = rule.id();
    let suppression = source.suppression_map();
    let suppressed = suppression.has_format_suppression();
    groups.retain(|g| {
        !g.is_empty() && (!suppressed || !g.iter().any(|e| suppression.suppresses(e, rule_id)))
    });
    groups
}

/// Lints `source` through every rule in `rules`, dropping the
/// diagnostics a format-suppressed span covers and the ones a
/// `# prose: ignore[<id>]` directive names on their own line, then
/// adds what each rule's audit reports over the run. A diagnostic on a
/// line a skip covers and no `ignore` sits on closes on the directive
/// that would hold it out.
pub(super) fn settled_lints(rules: &[Box<dyn Rule>], source: &Source) -> Vec<Diagnostic> {
    let suppression = source.suppression_map();
    let directives = suppression.has_lint_suppression() || !suppression.skips().is_empty();
    let mut silenced = Vec::new();
    let mut lints: Vec<Diagnostic> = rules
        .iter()
        .flat_map(|rule| rule.lint(source))
        .filter(|d| !suppression.intersects(d.range))
        .filter_map(|d| {
            if !directives {
                return Some(d);
            }
            let line = source.line_index(d.start());
            match suppression.ignore_at(line) {
                Some(ignore) if ignore.named.entry.matches(d.rule) => {
                    silenced.push((line, d.rule));
                    None
                }
                None if suppression.skipped(source.text().full_line_range(d.start())) => {
                    Some(with_ignore_help(d))
                }
                _ => Some(d),
            }
        })
        .collect();
    let held = Held::new(source, rules, silenced);
    lints.extend(rules.iter().flat_map(|rule| rule.audit(&held)));
    lints
}

/// `diagnostic` with its message closing on the `# prose: ignore[<id>]`
/// that holds it out.
fn with_ignore_help(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.message = format!(
        "{}. A `skip` holds out rewrites alone, so holding this finding out takes \
         `# prose: ignore[{}]` on its line",
        diagnostic.message, diagnostic.rule,
    );
    diagnostic
}

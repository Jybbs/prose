//! Parsing of the format-suppression namespace: `# prose: off` /
//! `on` / `skip` and their `skip[rule]` form.

use std::collections::HashSet;

use ruff_python_trivia::{PythonWhitespace, SuppressionKind};

use crate::rule::RuleId;

use super::parse_common::{after_prose_prefix, parse_bracketed_rule_list};

/// Result of `classify_format_directive`. `Kind` carries an upstream
/// or `# prose:`-prefixed off/on/skip directive that drives the span
/// machinery, whereas `SkipRules` carries the rule-id list parsed from
/// `# prose: skip[<id>[, <id>...]]`.
pub(super) enum FormatDirective {
    Kind(SuppressionKind),
    SkipRules(HashSet<RuleId>),
}

/// Classifies `comment` against the three format-suppression
/// namespaces. The `# prose:` namespace is tried first, falling
/// through to `SuppressionKind::from_comment` for the `# fmt:` and
/// `# yapf:` aliases. `# prose: skip[<id>...]` returns `SkipRules`,
/// `# prose: off|on|skip` and the alias forms return `Kind`. Multiple
/// `# prose: skip[<id>]` chunks on one comment range union their ids.
pub(super) fn classify_format_directive(comment: &str) -> Option<FormatDirective> {
    comment
        .split('#')
        .skip(1)
        .filter_map(parse_prose_format)
        .reduce(|mut acc, next| {
            if let (FormatDirective::SkipRules(a), FormatDirective::SkipRules(b)) = (&mut acc, next)
            {
                a.extend(b);
            }
            acc
        })
        .or_else(|| SuppressionKind::from_comment(comment).map(FormatDirective::Kind))
}

/// Parses the post-`#` body of a `prose: off`, `prose: on`, `prose:
/// skip`, or `prose: skip[<id>...]` directive. Returns `None` for any
/// other shape, leaving the caller to try the `# fmt:` / `# yapf:`
/// fallback.
fn parse_prose_format(after_hash: &str) -> Option<FormatDirective> {
    let body = after_prose_prefix(after_hash)?;
    if let Some(rest) = body.strip_prefix("skip").map(str::trim_whitespace) {
        if rest.is_empty() {
            return Some(FormatDirective::Kind(SuppressionKind::Skip));
        }
        return parse_bracketed_rule_list(rest).map(FormatDirective::SkipRules);
    }
    match body {
        "off" => Some(FormatDirective::Kind(SuppressionKind::Off)),
        "on" => Some(FormatDirective::Kind(SuppressionKind::On)),
        _ => None,
    }
}

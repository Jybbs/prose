//! Parsing of the format-suppression namespace: `# prose: off` /
//! `on` / `skip` and their `skip[<id>]` form.

use ruff_python_trivia::{PythonWhitespace, SuppressionKind};
use rustc_hash::FxHashSet;

use super::parse_common::parse_bracketed_rule_list;
use crate::rule::RuleId;

/// One format directive `directives` reads off a comment. `Kind`
/// carries an upstream or `# prose:`-prefixed off/on/skip directive
/// that drives the span machinery, whereas `SkipRules` carries the
/// rule-id list parsed from `# prose: skip[<id>[, <id>...]]`.
#[derive(Debug)]
pub(super) enum FormatDirective {
    Kind(SuppressionKind),
    SkipRules(FxHashSet<RuleId>),
}

/// Parses the body past a `prose:` prefix as `off`, `on`, `skip`, or
/// `skip[<id>...]`. Returns `None` for any other shape.
pub(super) fn parse_format(body: &str) -> Option<FormatDirective> {
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

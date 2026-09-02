//! Parsing of the format-suppression namespace: `# prose: off` /
//! `on` / `skip` and their `skip[<id>]` form.

use ruff_python_trivia::SuppressionKind;
use rustc_hash::FxHashSet;

use super::{lint_directive::RuleEntry, parse_common::parse_entry};
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
    if let Some(entry) = parse_entry(body, "skip") {
        return Some(match entry {
            RuleEntry::All => FormatDirective::Kind(SuppressionKind::Skip),
            RuleEntry::Specific(ids) => FormatDirective::SkipRules(ids),
        });
    }
    match body {
        "off" => Some(FormatDirective::Kind(SuppressionKind::Off)),
        "on" => Some(FormatDirective::Kind(SuppressionKind::On)),
        _ => None,
    }
}

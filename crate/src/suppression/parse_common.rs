//! Shared directive-parsing primitives: the `# prose:` prefix scan
//! and the bracketed rule-list reader.

use ruff_python_trivia::PythonWhitespace;
use rustc_hash::FxHashSet;

use super::lint_directive::RuleEntry;
use crate::rules::RuleId;

/// Strips the leading `prose:` marker from `after_hash` and returns
/// the trimmed body. Returns `None` for any other shape.
pub(super) fn after_prose_prefix(after_hash: &str) -> Option<&str> {
    after_hash
        .trim_whitespace()
        .strip_prefix("prose:")
        .map(str::trim_whitespace)
}

/// Parses the rule-id body of a `[<id>[, <id>...]]` suffix into the
/// set of rule ids. Returns `None` when the brackets are missing
/// or malformed. Unknown rule ids are silently dropped.
fn parse_bracketed_rule_list(body: &str) -> Option<FxHashSet<RuleId>> {
    Some(
        body.strip_prefix('[')?
            .strip_suffix(']')?
            .split(',')
            .filter_map(|part| part.trim_whitespace().parse::<RuleId>().ok())
            .collect(),
    )
}

/// The entry `body` spells past `keyword`, `All` for the bare keyword
/// and `Specific` for a bracketed list. `None` for any other shape.
pub(super) fn parse_entry(body: &str, keyword: &str) -> Option<RuleEntry> {
    let rest = body.strip_prefix(keyword)?.trim_whitespace();
    if rest.is_empty() {
        return Some(RuleEntry::All);
    }
    parse_bracketed_rule_list(rest).map(RuleEntry::Specific)
}

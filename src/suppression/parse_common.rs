//! Shared directive-parsing primitives: the `# prose:` prefix scan
//! and the bracketed rule-list reader.

use std::collections::HashSet;

use ruff_python_trivia::PythonWhitespace;

use crate::rule::RuleId;

/// Strips the leading `prose:` marker from `after_hash` and returns
/// the trimmed body. Returns `None` for any other shape.
pub(super) fn after_prose_prefix(after_hash: &str) -> Option<&str> {
    after_hash
        .trim_whitespace()
        .strip_prefix("prose:")
        .map(str::trim_whitespace)
}

/// Parses the rule-id body of a `[<id>[, <id>...]]` suffix into a
/// `RuleEntry::Specific`. Returns `None` when the brackets are missing
/// or malformed. Unknown rule ids are silently dropped.
pub(super) fn parse_bracketed_rule_list(body: &str) -> Option<HashSet<RuleId>> {
    Some(
        body.strip_prefix('[')?
            .strip_suffix(']')?
            .split(',')
            .filter_map(|part| part.trim_whitespace().parse::<RuleId>().ok())
            .collect(),
    )
}

//! Parsing of the lint-suppression namespace: `# prose: ignore` and
//! its `ignore[rule]` form, plus the per-line rule entry it records.

use std::collections::HashSet;

use ruff_python_trivia::PythonWhitespace;

use crate::rule::RuleId;

use super::parse_common::{after_prose_prefix, parse_bracketed_rule_list};

/// One line's parsed `# prose: ignore` or `# prose: skip[<id>]`
/// directive.
#[derive(Debug)]
pub(super) enum RuleEntry {
    /// Bare `# prose: ignore`. Suppresses every rule on the line.
    All,
    /// `# prose: ignore[<id>[, <id>...]]` or `# prose: skip[<id>[,
    /// <id>...]]`. Unknown ids are dropped.
    Specific(HashSet<RuleId>),
}

impl RuleEntry {
    /// Returns `true` when `self` suppresses `rule`. `All` matches
    /// every id, `Specific` matches only listed ids.
    pub(super) fn matches(&self, rule: RuleId) -> bool {
        match self {
            Self::All => true,
            Self::Specific(rules) => rules.contains(&rule),
        }
    }

    /// Folds `incoming` into `self`. `All` widens any prior `Specific`,
    /// and a second `Specific` unions its ids into the first.
    pub(super) fn merge(&mut self, incoming: Self) {
        match (&mut *self, incoming) {
            (Self::All, _) => {}
            (slot @ Self::Specific(_), Self::All) => *slot = Self::All,
            (Self::Specific(rules), Self::Specific(more)) => rules.extend(more),
        }
    }
}

impl Default for RuleEntry {
    fn default() -> Self {
        Self::Specific(HashSet::new())
    }
}

/// Splits `comment` at each `#` boundary, parsing each chunk as a
/// `prose: ignore` directive and folding successful hits through
/// `RuleEntry::merge`. Catches nested forms like `# my note # prose:
/// ignore` and multi-directive lines like `# prose: ignore  # prose:
/// ignore[align-equals]`.
pub(super) fn find_prose_ignore(comment: &str) -> Option<RuleEntry> {
    comment
        .split('#')
        .skip(1)
        .filter_map(parse_prose_ignore)
        .reduce(|mut acc, next| {
            acc.merge(next);
            acc
        })
}

/// Parses the post-`#` body of a `prose: ignore`, `prose: ignore[<id>]`,
/// or `prose: ignore[<id>, <id>...]` directive. Returns `None` for any
/// other shape. Whitespace tolerated around `:`, `[`, `,`, and `]`.
/// Unknown rule ids inside the brackets are dropped.
fn parse_prose_ignore(after_hash: &str) -> Option<RuleEntry> {
    let body = after_prose_prefix(after_hash)?
        .strip_prefix("ignore")?
        .trim_whitespace();
    if body.is_empty() {
        return Some(RuleEntry::All);
    }
    parse_bracketed_rule_list(body).map(RuleEntry::Specific)
}

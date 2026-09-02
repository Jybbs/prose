//! Parsing of the lint-suppression namespace: `# prose: ignore` and
//! its `ignore[<id>]` form, plus the rule entry both the lint and the
//! format namespaces record.

use rustc_hash::FxHashSet;

use super::parse_common::parse_entry;
use crate::rule::RuleId;

/// The rule set one `# prose: ignore` or `# prose: skip[<id>]`
/// directive records, indexed per line for `ignore` and per skip span
/// for `skip`.
#[derive(Clone, Debug)]
pub(super) enum RuleEntry {
    /// Bare `# prose: ignore`. Suppresses every rule on the line.
    All,
    /// `# prose: ignore[<id>[, <id>...]]` or `# prose: skip[<id>[,
    /// <id>...]]`. Unknown ids are dropped.
    Specific(FxHashSet<RuleId>),
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
        Self::Specific(FxHashSet::default())
    }
}

/// Parses the body past a `prose:` prefix as `ignore`, `ignore[<id>]`,
/// or `ignore[<id>, <id>...]`. Returns `None` for any other shape.
/// Whitespace is tolerated around `[`, `,`, and `]`, and unknown rule
/// ids inside the brackets are dropped.
pub(super) fn parse_ignore(body: &str) -> Option<RuleEntry> {
    parse_entry(body, "ignore")
}

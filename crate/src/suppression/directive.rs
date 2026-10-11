//! The records a `skip` or `ignore` directive parses into: the rule
//! entry both namespaces share, the rules one comment's chunks name, and
//! the directive the suppression map keeps beside the comment carrying
//! it.

use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::rules::RuleId;

/// One comment's `skip` or `ignore` directive, the rules its chunks
/// name beside the comment carrying them.
#[derive(Clone, Debug)]
pub(crate) struct Directive {
    pub(super) comment: TextRange,
    pub(crate) named: Named,
}

impl Ranged for Directive {
    fn range(&self) -> TextRange {
        self.comment
    }
}

/// The rules one comment's `skip` or `ignore` chunks name, beside every
/// bracketed id among them that names no registered rule, as written.
#[derive(Clone, Debug, Default)]
pub(crate) struct Named {
    pub(crate) entry: RuleEntry,
    pub(crate) unknown: Vec<Box<str>>,
}

impl Named {
    /// The bare form, naming every rule.
    pub(super) fn all() -> Self {
        Self {
            entry: RuleEntry::All,
            unknown: Vec::new(),
        }
    }

    /// Folds a later chunk's `incoming` into `self`, widening the entry
    /// per [`RuleEntry::merge`] and keeping every unknown id either one
    /// spelled.
    pub(super) fn merge(&mut self, incoming: Self) {
        self.entry.merge(incoming.entry);
        self.unknown.extend(incoming.unknown);
    }
}

/// The rule set one `# prose: ignore` or `# prose: skip[<id>]`
/// directive records, indexed per line for `ignore` and per skip span
/// for `skip`.
#[derive(Clone, Debug)]
pub(crate) enum RuleEntry {
    /// Bare `# prose: ignore` or `# prose: skip`, naming every rule.
    All,
    /// `# prose: ignore[<id>[, <id>...]]` or `# prose: skip[<id>[,
    /// <id>...]]`, holding the ids that name a registered rule.
    Specific(FxHashSet<RuleId>),
}

impl RuleEntry {
    /// Returns `true` when `self` suppresses `rule`. `All` matches
    /// every id, `Specific` matches only listed ids.
    pub(crate) fn matches(&self, rule: RuleId) -> bool {
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

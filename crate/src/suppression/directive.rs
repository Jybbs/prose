//! The records a `skip` or `ignore` directive parses into: the rule
//! entry both namespaces share, the rules one comment's chunks name, and
//! the directive the suppression map keeps beside the comment carrying
//! it.

use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange};
use rustc_hash::FxHashSet;

use super::parse::chunks;
use crate::rules::RuleId;

/// One comment's `skip` or `ignore` directive, the rules its chunks
/// name beside the comment carrying them.
#[derive(Clone, Debug)]
pub(crate) struct Directive {
    pub(super) comment: TextRange,
    pub(crate) named: Named,
}

impl Directive {
    /// The ranges of `source_text` whose deletion removes this
    /// directive's skip chunks, sorted and disjoint. A comment holding
    /// nothing but skip chunks goes whole with the whitespace before it,
    /// or with its line where it stands on a line of its own, whereas a
    /// run of skip chunks beside another chunk goes with the whitespace
    /// separating the two.
    pub(crate) fn skip_removal(&self, source_text: &str) -> Vec<TextRange> {
        let chunks = chunks(&source_text[self.comment]);
        if chunks.iter().all(|&(_, skip)| skip) {
            let line = source_text.full_line_range(self.comment.start());
            let lead = &source_text[TextRange::new(line.start(), self.comment.start())];
            if lead.trim_whitespace().is_empty() {
                return vec![line];
            }
            let code_end = line.start() + lead.trim_whitespace_end().text_len();
            return vec![TextRange::new(code_end, self.comment.end())];
        }
        let runs: Vec<&[(TextRange, bool)]> = chunks.chunk_by(|a, b| a.1 == b.1).collect();
        let end_of = |run: &[(TextRange, bool)]| run[run.len() - 1].0.end();
        runs.iter()
            .enumerate()
            .filter(|(_, run)| run[0].1)
            .map(|(index, run)| {
                let removed = match index.checked_sub(1) {
                    Some(before) => TextRange::new(end_of(runs[before]), end_of(run)),
                    None => TextRange::new(run[0].0.start(), runs[1][0].0.start()),
                };
                removed + self.comment.start()
            })
            .collect()
    }
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

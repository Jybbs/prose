//! Sheds a redundant grouping parenthesis pair, the pair whose removal
//! leaves the parse unchanged, so a precedence pair, a generator, a
//! walrus binding, and a one-element tuple all survive. A wrapped pair
//! folds onto one line when the bare form fits the budget, one whose
//! breaks a bracket inside it holds sheds in place, and `candidate`
//! finds the pairs while `measure` answers where each one lands.

use std::cmp::Reverse;

use ruff_diagnostics::Edit;
use ruff_text_size::{TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        edit::{insert_edit, singleton_groups},
        reseat::push_reseat_edits,
        walk::{Descent, filter_map_over_parented_exprs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod flush;
mod measure;
mod plan;

use flush::Flush;
use plan::{Candidate, candidate, outermost_calls};

pub(crate) struct ShedParentheses {
    code_line_length: usize,
    reflows_calls: bool,
}

impl ShedParentheses {
    pub(crate) const MESSAGE: &'static str = "shed a redundant grouping parenthesis pair";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            reflows_calls: config.rules.reflow_calls.enabled,
        }
    }
}

impl Rule for ShedParentheses {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut candidates =
            filter_map_over_parented_exprs(source.ast(), Descent::Into, |expr, parent| {
                candidate(source, expr, parent)
            });
        candidates.sort_unstable_by_key(|c| (c.pair.start(), Reverse(c.pair.end())));
        let calls = if self.reflows_calls {
            outermost_calls(source)
        } else {
            Vec::new()
        };
        let mut shedder = Shedder {
            calls,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            folds: Vec::new(),
            source,
        };
        shedder.shed(&candidates);
        singleton_groups(shedder.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Turns a candidate list into edits, walking it in source order so each
/// budget test reads the columns the preceding edits produce. `calls`
/// holds the outermost call ranges `reflow-calls` explodes where their
/// row overflows, empty where that rule is off.
pub(super) struct Shedder<'a> {
    pub(super) calls: Vec<TextRange>,
    pub(super) code_line_length: usize,
    pub(super) edits: Vec<Edit>,
    pub(super) folds: Vec<TextRange>,
    pub(super) source: &'a Source,
}

impl Shedder<'_> {
    /// Emits the deletions for every candidate, folding a wrapped pair
    /// whose joined line fits the budget. A candidate inside an open fold
    /// drops its parentheses alone, leaving that fold's own edits to
    /// close the break. A wrapped pair whose joined line overflows sheds
    /// in place when an enclosing bracket holds its breaks, re-seating
    /// the rows its parens moved, and holds otherwise.
    fn shed(&mut self, candidates: &[Candidate]) {
        for candidate in candidates {
            let Candidate { inner, pair, .. } = *candidate;
            self.folds.retain(|fold| fold.contains_range(pair));
            let collapsing = !self.folds.is_empty();
            let mut folding = !collapsing && self.source.contains_line_break(pair);
            let (open, close) = if collapsing {
                let paren = TextSize::new(1);
                (
                    TextRange::at(pair.start(), paren),
                    TextRange::at(pair.end() - paren, paren),
                )
            } else if folding && !self.fits(candidate, candidates) {
                let Some((open, close)) = self.shed_in_place_spans(candidate) else {
                    continue;
                };
                folding = false;
                (open, close)
            } else {
                (
                    TextRange::new(pair.start(), inner.start()),
                    TextRange::new(inner.end(), pair.end()),
                )
            };
            let removals = [
                Flush::removal(candidate.flush.before, open),
                Flush::removal(candidate.flush.after, close),
            ];
            if !collapsing && !folding {
                push_reseat_edits(self.source, &removals, &mut self.edits);
            }
            for removal in removals {
                insert_edit(&mut self.edits, removal);
            }
            if folding {
                self.push_fold_edits(inner);
                self.folds.push(pair);
            }
        }
    }
}

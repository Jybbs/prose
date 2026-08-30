//! Reflows a redundant grouping parenthesis pair against the line
//! budget, the pair whose removal leaves both the parse and the
//! grouping a reader sees unchanged, so a precedence pair, a generator,
//! a walrus binding, a one-element tuple, and a pair holding one
//! boolean operator inside a chain at the other all survive. A wrapped
//! pair folds onto one line when the bare form fits, one holding an
//! over-budget operator chain breaks across rows one operand per row,
//! and one whose interior breaks sit inside a bracket of its own sheds
//! in place. `plan` finds the pairs, `chain` divides an interior into
//! operands, `measure` answers whether each direction fits, and
//! `render` builds the broken text.

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    primitives::{
        edit::{insert_edit, singleton_groups},
        fracture::outermost,
        reseat::push_reseat_edits,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod chain;
mod flush;
mod measure;
mod plan;
mod render;

use plan::{Candidate, candidates, outermost_calls};

pub(crate) struct ReflowParentheses {
    code_line_length: usize,
    reflows_calls: bool,
}

impl ReflowParentheses {
    pub(crate) const MESSAGE: &'static str =
        "reflow a redundant grouping parenthesis pair against the line budget";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            reflows_calls: config.rules.reflow_calls.enabled,
        }
    }
}

impl Rule for ReflowParentheses {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let candidates = candidates(source);
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
        singleton_groups(outermost(shedder.edits))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Turns a candidate list into edits, walking it in source order so each
/// budget test reads the columns the preceding edits produce. `calls`
/// holds the outermost call ranges `reflow-calls` explodes where their
/// row overflows, empty where that rule is off.
struct Shedder<'a> {
    calls: Vec<TextRange>,
    code_line_length: usize,
    edits: Vec<Edit>,
    folds: Vec<TextRange>,
    source: &'a Source,
}

impl Shedder<'_> {
    /// Emits the edits for every candidate, folding a wrapped pair whose
    /// joined line fits the budget. A candidate inside an open fold
    /// drops its parentheses alone, leaving that fold's own edits to
    /// close the break. A pair whose joined line overflows breaks across
    /// rows where it holds an operator chain, sheds in place where an
    /// enclosing bracket holds its breaks, re-seating the rows its
    /// parens moved, and holds otherwise.
    fn shed(&mut self, candidates: &[Candidate]) {
        for candidate in candidates {
            let Candidate { inner, pair, .. } = *candidate;
            self.folds.retain(|fold| fold.contains_range(pair));
            let collapsing = !self.folds.is_empty();
            if !collapsing
                && self.overflows(candidate, candidates)
                && self.push_break_edits(candidate, candidates)
            {
                continue;
            }
            if !candidate.sheds {
                continue;
            }
            let mut folding = !collapsing && self.source.contains_line_break(pair);
            let removals = if collapsing {
                candidate.lone_paren_removals()
            } else if folding && !self.fits(candidate, candidates) {
                let Some((open, close)) = self.shed_in_place_spans(candidate) else {
                    continue;
                };
                folding = false;
                candidate.flush.flanking(open, close)
            } else {
                candidate.paren_removals()
            };
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

//! Sheds a backslash line continuation. A continuation a bracket
//! already spans drops the backslash and keeps its break, one outside
//! every bracket rejoins onto a single line where the joined line fits
//! the budget, and otherwise the outermost expression spanning the
//! break takes a parenthesis pair to carry the split, two runs spanned
//! by one expression taking one pair between them. A backslash the
//! lexer folded into a continued indentation is left alone.

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

mod gaps;
mod render;

use render::{join_edits, join_text, joined_width, stripped_edit, wrap_edits};

use gaps::{Gap, continuation_gaps, ends_atom, shares_a_run, stripped_gap};

use crate::{
    config::Config,
    primitives::{
        edit::{apply_inline_edits, narrowed_replacement},
        range::blocks_span,
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ShedBackslashContinuations {
    code_line_length: usize,
}

impl ShedBackslashContinuations {
    pub(crate) const MESSAGE: &'static str = "shed a backslash line continuation";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
        }
    }

    /// The fix for one run of unbracketed gaps, the join where the
    /// merged line fits the budget, and otherwise the parenthesized
    /// break wherever an expression spans the run.
    fn shed_run(&self, source: &Source, run: &[Gap]) -> Shed {
        let span = blocks_span(run);
        let joined = join_edits(source, run);
        if joined_width(source, span, &joined) <= self.code_line_length {
            return Shed::Joined(joined);
        }
        wrap_edits(source, span, run).map_or(Shed::Joined(joined), |(node, stripped)| {
            Shed::Wrapped { node, stripped }
        })
    }
}

impl Rule for ShedBackslashContinuations {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let gaps = continuation_gaps(source);
        let mut groups = Vec::new();
        let mut wraps: Vec<(TextRange, Vec<Edit>)> = Vec::new();
        for run in gaps.chunk_by(|earlier, later| shares_a_run(source, earlier, later)) {
            match run {
                [gap] if gap.bracketed => {
                    groups.extend(stripped_edit(source, gap.range).map(|edit| vec![edit]));
                }
                _ => match self.shed_run(source, run) {
                    Shed::Joined(edits) => groups.push(edits),
                    Shed::Wrapped { node, stripped } => {
                        match wraps.iter_mut().find(|(wrapped, _)| *wrapped == node) {
                            Some((_, edits)) => edits.extend(stripped),
                            None => wraps.push((node, stripped)),
                        }
                    }
                },
            }
        }
        for (node, mut edits) in wraps {
            edits.insert(0, Edit::insertion("(".to_owned(), node.start()));
            edits.push(Edit::insertion(")".to_owned(), node.end()));
            groups.push(edits);
        }
        groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The fix one unbracketed run takes: its gaps joined onto one line, or
/// its backslashes stripped inside a pair wrapping `node`, the
/// outermost expression spanning the run.
enum Shed {
    Joined(Vec<Edit>),
    Wrapped {
        node: TextRange,
        stripped: Vec<Edit>,
    },
}

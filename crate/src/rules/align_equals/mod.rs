//! Vertically aligns `=` across runs of same-indent, line-adjacent
//! assignments, annotated parameter defaults, and an exploded call's
//! keyword arguments, aligning a run only when its rows share a column
//! baseline. Chained assignments, initializer-less annotations, and
//! single-line signatures or calls are skipped. Every aligned row
//! reads as `name = value`, the name side padding to the shared column
//! and collapsing to one space where no column is reached, and the
//! value side collapsing to one space after the operator unless the
//! value falls on a later line. A keyword condensed onto a line with
//! another argument keeps its tight `name=value`, `+=` places `+` one
//! column before the shared `=`, and parameter widths reflect the
//! post-`align_colons` source.

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::Visitor as AstVisitor;

use crate::{
    config::Config,
    primitives::{aligner, equal_targets, walk::walk_stmt},
    rules::{Rule, RuleId},
    source::Source,
};

mod walk;

use walk::{Run, Visitor};

#[derive(Debug)]
pub(crate) struct AlignEquals {
    settings: aligner::Settings,
}

impl AlignEquals {
    pub(crate) const MESSAGE: &'static str = "align consecutive `=` operators";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: config.equals_settings(),
        }
    }
}

impl Rule for AlignEquals {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            runs: Vec::new(),
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        let members = visitor.runs.iter().flat_map(Run::members).copied();
        let widenings = aligner::Widenings::of(source, self.settings, members);
        visitor.walker.set_widenings(widenings);
        for run in std::mem::take(&mut visitor.runs) {
            match run {
                Run::Buffered(group) => visitor.walker.emit_group_or_buffer(&group),
                Run::Candidate(group) => visitor.walker.emit_if_candidate(&group),
            }
        }
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

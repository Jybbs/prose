//! Collapses each `match` arm to a one-line `case PATTERN : EXPR`
//! and aligns the `:` column across arms whose body is a single
//! collapsible statement on one source line. A disqualifying arm
//! (multi-statement body, compound-statement body, multi-line body,
//! or a comment between the `:` and the body) ends the run with its
//! `:` flush against the pattern. An arm whose collapsed line would
//! exceed `Config::code_line_length`, alone or padded to the run's
//! column, ends the run the same way and splits onto two lines where
//! it sits folded. Nested matches recurse.

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::StatementVisitor;

use crate::{
    config::Config,
    primitives::{
        aligner, colon_targets,
        comments::{Settling, trailing_comment},
        layout::item_indent,
    },
    rules::{Rule, RuleId},
    source::Source,
};

mod walk;

use walk::Visitor;

#[derive(Debug)]
pub(crate) struct AlignMatchCase {
    code_line_length: usize,
    settings: aligner::Settings,
    settling: Settling,
}

impl AlignMatchCase {
    pub(crate) const MESSAGE: &'static str = "align consecutive `case` colons";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            settings: aligner::Settings::from(&config.rules.align_match_case)
                .with_singleton_strip(),
            settling: config.comment_settling(),
        }
    }
}

impl Rule for AlignMatchCase {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            code_line_length: self.code_line_length,
            settling: self.settling,
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

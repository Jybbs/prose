//! Collapses each `match` arm to a one-line `case PATTERN : EXPR`
//! and aligns the `:` column across arms whose body is a single
//! collapsible statement on one source line. A disqualifying arm
//! (multi-statement body, compound-statement body, multi-line body,
//! or a comment between the `:` and the body) breaks alignment into
//! sub-groups on either side. An arm whose collapsed form would
//! exceed `Config::code_line_length` also disqualifies, and any
//! such arm that sits on one source line splits so the body lands
//! on the next line. Nested matches recurse.

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::StatementVisitor;

mod walk;

use walk::Visitor;

use crate::{
    config::Config,
    primitives::{INDENT_STEP, aligner, colon_targets},
    rule::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct AlignMatchCase {
    code_line_length: usize,
    settings: aligner::Settings,
}

impl AlignMatchCase {
    pub(crate) const MESSAGE: &'static str = "align match-case colons";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            settings: aligner::Settings::from(&config.rules.align_match_case)
                .with_singleton_strip(),
        }
    }
}

impl Rule for AlignMatchCase {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            code_line_length: self.code_line_length,
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

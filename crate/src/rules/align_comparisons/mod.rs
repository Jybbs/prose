//! Aligns comparison operators vertically across the operands of a
//! multi-line `BoolOp`. Every `Expr::Compare` operand qualifies
//! regardless of left-side shape or comparison kind, with chained
//! compares anchoring on their first operator. Variable-width
//! operators right-align so each operator's last character sits in
//! the shared column. A non-comparison operand, a multi-line operand,
//! or a blank line in the gap breaks the run.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprBoolOp, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    primitives::{aligner, comparison::opening_token_kind, walk::walk_stmt},
    rules::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct AlignComparisons {
    settings: aligner::Settings,
}

impl AlignComparisons {
    pub(crate) const MESSAGE: &'static str = "align consecutive comparison operators";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: config.align_settings(&config.rules.align_comparisons, config.code_width()),
        }
    }
}

impl Rule for AlignComparisons {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Visitor<'a> {
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    fn process_bool_op(&mut self, bool_op: &ExprBoolOp) {
        let source = self.walker.source;
        if !source.contains_line_break(bool_op) {
            return;
        }
        let groups = aligner::adjacent_member_groups(source, &bool_op.values, true, |operand| {
            self.qualify(operand).into()
        });
        for group in &groups {
            self.walker.emit_unheld(group.iter().copied());
        }
    }

    fn qualify(&self, operand: &Expr) -> Option<aligner::Member> {
        let compare = operand.as_compare_expr()?;
        let op = *compare.ops.first()?;
        let comparator = compare.comparators.first()?;
        let member = aligner::line_anchored_member_between(
            self.walker.source,
            compare.left.range(),
            comparator.start(),
            opening_token_kind(op),
        )?;
        Some(member.with_op_width(op.as_str().len()))
    }
}

impl<'a> AstVisitor<'a> for Visitor<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::BoolOp(bool_op) = expr {
            self.process_bool_op(bool_op);
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        walk_stmt(self, stmt);
    }
}

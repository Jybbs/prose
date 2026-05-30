//! Aligns comparison operators vertically across the operands of a
//! multi-line `BoolOp`. Every `Expr::Compare` operand qualifies
//! regardless of left-side shape or comparison kind, with chained
//! compares anchoring on their first operator. Variable-width
//! operators right-align so each operator's last character sits in
//! the shared column. A non-comparison operand, a multi-line operand,
//! or a blank line in the gap breaks the run.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{walk_expr, Visitor as AstVisitor};
use ruff_python_ast::{CmpOp, Expr, ExprBoolOp};
use ruff_text_size::{Ranged, TextRange};

use crate::config::Config;
use crate::primitives::aligner;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct AlignComparisons {
    settings: aligner::Settings,
}

impl AlignComparisons {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_comparisons),
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
        if !self.walker.source.contains_line_break(bool_op) {
            return;
        }
        let mut groups: Vec<Vec<aligner::Member>> = Vec::new();
        let mut active: Option<TextRange> = None;
        for operand in &bool_op.values {
            let Some(member) = self.qualify(operand) else {
                active = None;
                continue;
            };
            let extends = active.is_some_and(|prev| {
                !self.walker.source.contains_line_break(prev)
                    && self.walker.source.line_index(operand.start())
                        == self.walker.source.line_index(prev.end()).saturating_add(1)
            });
            if extends {
                groups
                    .last_mut()
                    .expect("active implies groups non-empty")
                    .push(member);
            } else {
                groups.push(vec![member]);
            }
            active = Some(operand.range());
        }
        for group in &groups {
            self.walker.emit_unheld(group.iter().copied());
        }
    }

    fn qualify(&self, operand: &Expr) -> Option<aligner::Member> {
        let compare = operand.as_compare_expr()?;
        let op = *compare.ops.first()?;
        let comparator = compare.comparators.first()?;
        let member = aligner::line_anchored_member_at_kind(
            self.walker.source,
            TextRange::new(compare.left.end(), comparator.start()),
            cmp_op_anchor_token_kind(op),
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
}

/// Maps every `CmpOp` to the lexer token that anchors the operator's
/// column. Compound operators (`is not`, `not in`) anchor on the first
/// keyword, so the alignment math always treats the operator as
/// starting at the returned token.
fn cmp_op_anchor_token_kind(op: CmpOp) -> TokenKind {
    match op {
        CmpOp::Eq => TokenKind::EqEqual,
        CmpOp::Gt => TokenKind::Greater,
        CmpOp::GtE => TokenKind::GreaterEqual,
        CmpOp::In => TokenKind::In,
        CmpOp::Is | CmpOp::IsNot => TokenKind::Is,
        CmpOp::Lt => TokenKind::Less,
        CmpOp::LtE => TokenKind::LessEqual,
        CmpOp::NotEq => TokenKind::NotEqual,
        CmpOp::NotIn => TokenKind::Not,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(CmpOp::Eq, TokenKind::EqEqual)]
    #[case(CmpOp::Gt, TokenKind::Greater)]
    #[case(CmpOp::GtE, TokenKind::GreaterEqual)]
    #[case(CmpOp::In, TokenKind::In)]
    #[case(CmpOp::Is, TokenKind::Is)]
    #[case(CmpOp::IsNot, TokenKind::Is)]
    #[case(CmpOp::Lt, TokenKind::Less)]
    #[case(CmpOp::LtE, TokenKind::LessEqual)]
    #[case(CmpOp::NotEq, TokenKind::NotEqual)]
    #[case(CmpOp::NotIn, TokenKind::Not)]
    fn cmp_op_anchor_token_kind_covers_every_variant(
        #[case] op: CmpOp,
        #[case] expected: TokenKind,
    ) {
        assert_eq!(cmp_op_anchor_token_kind(op), expected);
    }
}

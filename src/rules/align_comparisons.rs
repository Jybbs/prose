//! Aligns comparison operators vertically across the operands of a
//! multi-line `BoolOp`. Every `Expr::Compare` operand qualifies
//! regardless of left-side shape or comparison kind, with chained
//! compares anchoring on their first operator. Variable-width
//! operators right-align so each operator's last character sits in
//! the shared column. A non-comparison operand, a multi-line operand,
//! or a blank line in the gap breaks the run.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{walk_expr, Visitor};
use ruff_python_ast::{CmpOp, Expr, ExprBoolOp};
use ruff_text_size::{Ranged, TextRange, TextSize};

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
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut walker = Walker {
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        walker.visit_body(&source.ast().body);
        walker.edits
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl Walker<'_> {
    /// Returns `true` when `prev_end` and `next_start` sit on directly
    /// consecutive source lines. Directive comments (`# fmt: off`,
    /// `# prose: skip[...]`) are handled by the pipeline's
    /// `SuppressionMap` filter on emitted edits, so the rule itself
    /// stays comment-agnostic.
    fn is_operand_line_adjacent(&self, prev_end: TextSize, next_start: TextSize) -> bool {
        self.source.line_index(next_start) == self.source.line_index(prev_end).saturating_add(1)
    }

    fn process_bool_op(&mut self, bool_op: &ExprBoolOp) {
        if !self.source.contains_line_break(bool_op) {
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
                !self.source.contains_line_break(prev)
                    && self.is_operand_line_adjacent(prev.end(), operand.start())
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
            if aligner::is_alignment_candidate(group) {
                aligner::emit_group(self.source, group, self.settings, &mut self.edits);
            }
        }
    }

    fn qualify(&self, operand: &Expr) -> Option<aligner::Member> {
        let compare = operand.as_compare_expr()?;
        let op = *compare.ops.first()?;
        let comparator = compare.comparators.first()?;
        let member = aligner::line_anchored_member_at_kind(
            self.source,
            TextRange::new(compare.left.end(), comparator.start()),
            cmp_op_anchor_token_kind(op),
        )?;
        Some(member.with_op_width(op.as_str().len()))
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
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
    use super::*;

    #[test]
    fn cmp_op_anchor_token_kind_covers_every_variant() {
        for (op, expected) in [
            (CmpOp::Eq, TokenKind::EqEqual),
            (CmpOp::Gt, TokenKind::Greater),
            (CmpOp::GtE, TokenKind::GreaterEqual),
            (CmpOp::In, TokenKind::In),
            (CmpOp::Is, TokenKind::Is),
            (CmpOp::IsNot, TokenKind::Is),
            (CmpOp::Lt, TokenKind::Less),
            (CmpOp::LtE, TokenKind::LessEqual),
            (CmpOp::NotEq, TokenKind::NotEqual),
            (CmpOp::NotIn, TokenKind::Not),
        ] {
            assert_eq!(cmp_op_anchor_token_kind(op), expected);
        }
    }
}

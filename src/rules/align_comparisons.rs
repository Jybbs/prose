//! Aligns comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`)
//! vertically across the operands of a multi-line `BoolOp`. Operands
//! qualify when they are single-comparator `ExprCompare`s whose left
//! side is a `Name`, `Attribute`, or `Subscript`. A non-qualifying or
//! multi-line operand breaks the run, and operands with different
//! comparison operators form independent groups. Comparisons inside
//! list, set, dict, and generator comprehensions are skipped.

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
    /// Returns `true` when `prev_end` and `next_start` sit on
    /// consecutive source lines with no intervening comment.
    fn is_operand_line_adjacent(&self, prev_end: TextSize, next_start: TextSize) -> bool {
        !self
            .source
            .slice(TextRange::new(prev_end, next_start))
            .contains('#')
            && self.source.line_index(next_start)
                == self.source.line_index(prev_end).saturating_add(1)
    }

    fn process_bool_op(&mut self, bool_op: &ExprBoolOp) {
        if !self.source.contains_line_break(bool_op) {
            return;
        }
        let mut groups: Vec<Vec<aligner::Member>> = Vec::new();
        let mut active: Option<(TokenKind, TextRange)> = None;
        for operand in &bool_op.values {
            let Some((kind, member)) = self.qualify(operand) else {
                active = None;
                continue;
            };
            let extends = active.as_ref().is_some_and(|(prev_kind, prev)| {
                *prev_kind == kind
                    && !self.source.contains_line_break(prev)
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
            active = Some((kind, operand.range()));
        }
        for group in &groups {
            if aligner::is_alignment_candidate(group) {
                aligner::emit_group(self.source, group, self.settings, &mut self.edits);
            }
        }
    }

    fn qualify(&self, operand: &Expr) -> Option<(TokenKind, aligner::Member)> {
        let compare = operand.as_compare_expr()?;
        let [op] = *compare.ops else {
            return None;
        };
        let [comparator] = compare.comparators.as_ref() else {
            return None;
        };
        if !matches!(
            *compare.left,
            Expr::Attribute(_) | Expr::Name(_) | Expr::Subscript(_),
        ) {
            return None;
        }
        let kind = cmp_op_token_kind(op)?;
        let search = TextRange::new(compare.left.end(), comparator.start());
        let member = aligner::line_anchored_member_at_kind(self.source, search, kind)?;
        Some((kind, member))
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::BoolOp(bool_op) => self.process_bool_op(bool_op),
            Expr::DictComp(_) | Expr::Generator(_) | Expr::ListComp(_) | Expr::SetComp(_) => return,
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// Maps the `CmpOp` variants this rule aligns to their lexer token
/// kind. Identity and membership operators return `None`.
fn cmp_op_token_kind(op: CmpOp) -> Option<TokenKind> {
    Some(match op {
        CmpOp::Eq => TokenKind::EqEqual,
        CmpOp::Gt => TokenKind::Greater,
        CmpOp::GtE => TokenKind::GreaterEqual,
        CmpOp::Lt => TokenKind::Less,
        CmpOp::LtE => TokenKind::LessEqual,
        CmpOp::NotEq => TokenKind::NotEqual,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmp_op_token_kind_covers_six_aligned_operators() {
        for (op, expected) in [
            (CmpOp::Eq, TokenKind::EqEqual),
            (CmpOp::Gt, TokenKind::Greater),
            (CmpOp::GtE, TokenKind::GreaterEqual),
            (CmpOp::Lt, TokenKind::Less),
            (CmpOp::LtE, TokenKind::LessEqual),
            (CmpOp::NotEq, TokenKind::NotEqual),
        ] {
            assert_eq!(cmp_op_token_kind(op), Some(expected));
        }
    }

    #[test]
    fn cmp_op_token_kind_rejects_identity_and_membership() {
        for op in [CmpOp::In, CmpOp::Is, CmpOp::IsNot, CmpOp::NotIn] {
            assert!(cmp_op_token_kind(op).is_none());
        }
    }

    #[test]
    fn rule_id_is_align_comparisons() {
        let rule = AlignComparisons::from_config(&Config::default());
        assert_eq!(rule.id().as_str(), "align-comparisons");
    }
}

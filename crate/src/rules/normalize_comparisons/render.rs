//! Turns a rewrite plan into the edits that land it, the operator range
//! each rewrite covers, and the enclosing `not` a fold removes.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, CmpOp, Expr, ExprUnaryOp, helpers::is_constant_non_singleton};
use ruff_text_size::{Ranged, TextRange};

use super::plan::{Plan, Test};
use crate::{primitives::comparison::opening_token_kind, source::Source};

/// The edits `plan` calls for, the `not` deletion first, then the
/// operand swap, then the operator-token replacement. `None` where a
/// rewritten operator resolves to no token, which declines the whole
/// group rather than swapping operands around an operator that stayed
/// as authored.
pub(super) fn edits(
    source: &Source,
    test: Test<'_>,
    plan: &Plan,
    negated: Option<&ExprUnaryOp>,
) -> Option<Vec<Edit>> {
    let lead = source.paren_aware_range(test.left.into(), test.compare.into());
    let trail = source.paren_aware_range(test.right.into(), test.compare.into());
    let operator = if plan.op == test.op {
        None
    } else {
        Some(operator_range(
            source,
            TextRange::new(lead.end(), trail.start()),
            test.op,
        )?)
    };
    let mut edits = Vec::new();
    if let Some(unary) = negated.filter(|_| plan.drop_not) {
        let operand = source.paren_aware_range(test.compare.into(), unary.into());
        edits.push(Edit::range_deletion(TextRange::new(
            unary.start(),
            operand.start(),
        )));
    }
    if plan.flip {
        edits.push(Edit::range_replacement(
            source.slice(trail).to_owned(),
            lead,
        ));
        edits.push(Edit::range_replacement(
            source.slice(lead).to_owned(),
            trail,
        ));
    }
    edits.extend(operator.map(|range| Edit::range_replacement(plan.op.as_str().to_owned(), range)));
    Some(edits)
}

/// The identity operator a test settles on when either side is a `None`
/// literal, or `None` where neither is or the other side is a constant
/// `is` would not match.
pub(super) fn identity_op(op: CmpOp, left: &Expr, right: &Expr) -> Option<CmpOp> {
    let identity = match op {
        CmpOp::Eq => CmpOp::Is,
        CmpOp::NotEq => CmpOp::IsNot,
        _ => return None,
    };
    let other = if left.is_none_literal_expr() {
        right
    } else if right.is_none_literal_expr() {
        left
    } else {
        return None;
    };
    (!is_constant_non_singleton(other)).then_some(identity)
}

/// The `not` expression `parent` names, or `None` for every other node.
pub(super) fn negating_parent(parent: AnyNodeRef<'_>) -> Option<&ExprUnaryOp> {
    match parent {
        AnyNodeRef::ExprUnaryOp(unary) if unary.op.is_not() => Some(unary),
        _ => None,
    }
}

/// The range of the lone lexer token spelling `op` inside `gap`.
fn operator_range(source: &Source, gap: TextRange, op: CmpOp) -> Option<TextRange> {
    let kind = opening_token_kind(op);
    source
        .tokens()
        .in_range(gap)
        .iter()
        .find(|token| token.kind() == kind)
        .map(Ranged::range)
}

/// The operator a flipped test reads with, or `None` for the operators
/// an operand swap does not preserve.
pub(super) const fn reflected(op: CmpOp) -> Option<CmpOp> {
    match op {
        CmpOp::Eq => Some(CmpOp::Eq),
        CmpOp::Gt => Some(CmpOp::Lt),
        CmpOp::GtE => Some(CmpOp::LtE),
        CmpOp::Lt => Some(CmpOp::Gt),
        CmpOp::LtE => Some(CmpOp::GtE),
        CmpOp::NotEq => Some(CmpOp::NotEq),
        CmpOp::In | CmpOp::Is | CmpOp::IsNot | CmpOp::NotIn => None,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        rules::normalize_comparisons::tests::test_of,
        testing::{first_expr, parse},
    };

    #[rstest]
    #[case("value == None\n", Some(CmpOp::Is))]
    #[case("value != None\n", Some(CmpOp::IsNot))]
    #[case("None == value\n", Some(CmpOp::Is))]
    #[case("None == 5\n", None)]
    #[case("value == 5\n", None)]
    #[case("value < None\n", None)]
    fn identity_op_maps_an_equality_against_none(
        #[case] src: &str,
        #[case] expected: Option<CmpOp>,
    ) {
        let source = parse(src);
        let test = test_of(&source).expect("a two-operand comparison");
        assert_eq!(identity_op(test.op, test.left, test.right), expected);
    }

    #[test]
    fn negating_parent_declines_a_non_not_unary() {
        let source = parse("-x\n");
        let unary = first_expr(&source)
            .as_unary_op_expr()
            .expect("first statement is a unary op");

        assert!(negating_parent(unary.into()).is_none());
    }

    #[test]
    fn operator_range_declines_a_gap_holding_no_operator_token() {
        let source = parse("value == None\n");
        assert!(operator_range(&source, TextRange::default(), CmpOp::Eq).is_none());
    }

    #[rstest]
    #[case(CmpOp::Eq, Some(CmpOp::Eq))]
    #[case(CmpOp::Gt, Some(CmpOp::Lt))]
    #[case(CmpOp::GtE, Some(CmpOp::LtE))]
    #[case(CmpOp::In, None)]
    #[case(CmpOp::Is, None)]
    #[case(CmpOp::IsNot, None)]
    #[case(CmpOp::Lt, Some(CmpOp::Gt))]
    #[case(CmpOp::LtE, Some(CmpOp::GtE))]
    #[case(CmpOp::NotEq, Some(CmpOp::NotEq))]
    #[case(CmpOp::NotIn, None)]
    fn reflected_covers_every_variant(#[case] op: CmpOp, #[case] expected: Option<CmpOp>) {
        assert_eq!(reflected(op), expected);
    }
}

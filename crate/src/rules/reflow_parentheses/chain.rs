//! Divides an operator chain into the operands a break lands one per
//! row, reading the chain off its own outermost node and against the
//! pairs the same pass sheds.

use ruff_python_ast::{AnyNodeRef, Expr, ExprRef, OperatorPrecedence};
use ruff_text_size::{Ranged, TextRange};

use crate::source::Source;

/// Reports whether a pair around a given range comes off in this pass.
pub(super) type Sheds<'a> = &'a dyn Fn(TextRange) -> bool;

/// The operator level a break cuts at, every row of the break sharing
/// it.
#[derive(Clone, Copy)]
enum Level {
    /// Both boolean operators, an `and` nested inside an `or` reading
    /// as one run of rows.
    Boolean,
    /// One binary-operator precedence, leaving a tighter-binding
    /// operand whole on its row.
    Binary(OperatorPrecedence),
}

/// True for the three nodes a break divides into rows, read through
/// `ExprRef` so an ancestor reached as an `AnyNodeRef` answers the same
/// question a bare expression does.
pub(super) fn is_operator_chain(expr: ExprRef) -> bool {
    matches!(
        expr,
        ExprRef::BinOp(_) | ExprRef::BoolOp(_) | ExprRef::Compare(_)
    )
}

/// The operand ranges `expr`'s outermost operator chain divides into,
/// in source order. An operand `sheds` reports a pair around carries
/// the bare range and divides further where its own node joins at the
/// same level, so the division reads the text this pass leaves rather
/// than the text it was handed. `None` for an expression that is no
/// operator chain.
pub(super) fn operands(source: &Source, expr: &Expr, sheds: Sheds) -> Option<Vec<TextRange>> {
    let mut out = Vec::new();
    match expr {
        Expr::BinOp(binary) => {
            let level = Level::Binary(OperatorPrecedence::from(binary.op));
            for side in [binary.left.as_ref(), binary.right.as_ref()] {
                push_operands(source, side, expr.into(), level, sheds, &mut out);
            }
        }
        Expr::BoolOp(boolean) => {
            for value in &boolean.values {
                push_operands(source, value, expr.into(), Level::Boolean, sheds, &mut out);
            }
        }
        Expr::Compare(compare) => {
            let held = |operand: &Expr| operand_range(source, operand, expr.into(), sheds);
            out.push(held(compare.left.as_ref()));
            out.extend(compare.comparators.iter().map(held));
        }
        _ => return None,
    }
    Some(out)
}

/// The range `expr` occupies as an operand, widened to the pair around
/// it and narrowed back to the bare node where `sheds` reports that
/// pair comes off.
fn operand_range(source: &Source, expr: &Expr, parent: AnyNodeRef, sheds: Sheds) -> TextRange {
    let held = source.paren_aware_range(expr.into(), parent);
    if sheds(held) { expr.range() } else { held }
}

/// Pushes `expr`'s own operand ranges onto `out`, descending through a
/// node joining its children at `level` and pushing the range whole
/// otherwise. A pair the source wrote around `expr` holds the node on
/// one row, except where `sheds` reports the pair comes off, leaving
/// the node to divide as the shed text would.
fn push_operands(
    source: &Source,
    expr: &Expr,
    parent: AnyNodeRef,
    level: Level,
    sheds: Sheds,
    out: &mut Vec<TextRange>,
) {
    let held = operand_range(source, expr, parent, sheds);
    let bare = held == expr.range();
    if bare {
        match (expr, level) {
            (Expr::BinOp(binary), Level::Binary(precedence))
                if OperatorPrecedence::from(binary.op) == precedence =>
            {
                for side in [binary.left.as_ref(), binary.right.as_ref()] {
                    push_operands(source, side, expr.into(), level, sheds, out);
                }
                return;
            }
            (Expr::BoolOp(boolean), Level::Boolean) => {
                for value in &boolean.values {
                    push_operands(source, value, expr.into(), level, sheds, out);
                }
                return;
            }
            _ => {}
        }
    }
    out.push(held);
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

    /// `src`'s chain divided with no pair shedding, each operand as the
    /// source spells it.
    fn divided(src: &str) -> Option<Vec<String>> {
        let source = parse(src);
        let expr = first_expr(&source);
        let holds = |_: TextRange| false;
        operands(&source, expr, &holds)
            .map(|ranges| ranges.iter().map(|r| source.slice(*r).to_owned()).collect())
    }

    /// `src`'s chain divided with every grouping pair shedding.
    fn divided_shedding(src: &str) -> Option<Vec<String>> {
        let source = parse(src);
        let expr = first_expr(&source);
        let all = |_: TextRange| true;
        operands(&source, expr, &all)
            .map(|ranges| ranges.iter().map(|r| source.slice(*r).to_owned()).collect())
    }

    #[rstest]
    #[case("value", false)]
    #[case("helper(a, b)", false)]
    #[case("a and b", true)]
    #[case("a + b", true)]
    #[case("a < b", true)]
    fn is_operator_chain_covers_the_three_dividing_nodes(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(is_operator_chain(first_expr(&source).into()), expected);
    }

    #[rstest]
    #[case("value")]
    #[case("helper(a, b)")]
    #[case("[a, b]")]
    #[case("a if b else c")]
    #[case("-a")]
    fn operands_declines_an_expression_that_is_no_chain(#[case] src: &str) {
        assert!(divided(src).is_none());
    }

    #[rstest]
    #[case("a and b and c", &["a", "b", "c"])]
    #[case("a and b or c and d", &["a", "b", "c", "d"])]
    #[case("a == 1 and b != 2", &["a == 1", "b != 2"])]
    #[case("a < b < c", &["a", "b", "c"])]
    #[case("a + b + c", &["a", "b", "c"])]
    #[case("a + b * c + d", &["a", "b * c", "d"])]
    #[case("a + (b + c) + d", &["a", "(b + c)", "d"])]
    #[case("a | b | c", &["a", "b", "c"])]
    #[case("not a and b", &["not a", "b"])]
    fn operands_divide_a_chain_at_its_outermost_level(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let divided = divided(src).expect("the source holds a chain");
        assert_eq!(
            divided.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }

    #[rstest]
    #[case("a or (b and c)", &["a", "b", "c"])]
    #[case("a + (b + c) + d", &["a", "b", "c", "d"])]
    #[case("a and (b == 1)", &["a", "b == 1"])]
    fn operands_divide_through_a_pair_the_pass_sheds(#[case] src: &str, #[case] expected: &[&str]) {
        let divided = divided_shedding(src).expect("the source holds a chain");
        assert_eq!(
            divided.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }
}

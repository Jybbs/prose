//! Divides an operator chain into the operands a break lands one per
//! row, reading the chain off its own outermost node and against the
//! pairs the same pass sheds.

use ruff_python_ast::{AnyNodeRef, Expr, ExprRef, OperatorPrecedence};
use ruff_text_size::{Ranged, TextRange};

use crate::source::Source;

/// One operand of a divided chain, carrying the range it occupies and
/// the spelling of the operator joining it to the operand ahead of it.
/// The chain's first operand joins nothing and leaves `lead` as `None`.
pub(super) struct Operand {
    pub(super) lead: Option<&'static str>,
    pub(super) range: TextRange,
}

/// Reports whether a pair around a given range comes off in this pass.
pub(super) type Sheds<'a> = &'a dyn Fn(TextRange) -> bool;

/// The context one chain division carries down its recursion, fixed
/// for the whole walk while the node, its parent, and the operator
/// leading it change per step.
struct Division<'a> {
    level: OperatorPrecedence,
    sheds: Sheds<'a>,
    source: &'a Source,
}

impl<'a> Division<'a> {
    /// The division cutting at `level`, reading `source` against the
    /// pairs `sheds` reports coming off.
    fn at(level: OperatorPrecedence, sheds: Sheds<'a>, source: &'a Source) -> Self {
        Self {
            level,
            sheds,
            source,
        }
    }

    /// The range `expr` occupies as an operand, widened to the pair
    /// around it and narrowed back to the bare node where `sheds`
    /// reports that pair comes off.
    fn operand_range(&self, expr: &Expr, parent: AnyNodeRef) -> TextRange {
        let held = self.source.paren_aware_range(expr.into(), parent);
        if (self.sheds)(held) {
            expr.range()
        } else {
            held
        }
    }

    /// Pushes `expr`'s own operands onto `out`, descending through a
    /// node joining its children at this division's level and pushing
    /// the range whole otherwise. `lead` is the operator joining `expr`
    /// to what precedes it, which the first operand of a descent
    /// inherits and each later one takes from the node it descended
    /// through. A pair the source wrote around `expr` holds the node on
    /// one row, except where the pair comes off, leaving the node to
    /// divide as the shed text would.
    fn push(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        lead: Option<&'static str>,
        out: &mut Vec<Operand>,
    ) {
        let held = self.operand_range(expr, parent);
        if held == expr.range() {
            match expr {
                Expr::BinOp(binary) if OperatorPrecedence::from(binary.op) == self.level => {
                    self.push(&binary.left, expr.into(), lead, out);
                    self.push(&binary.right, expr.into(), Some(binary.op.as_str()), out);
                    return;
                }
                _ => {}
            }
        }
        out.push(Operand { lead, range: held });
    }
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

/// The operands `expr`'s outermost operator chain divides into,
/// in source order. An operand `sheds` reports a pair around carries
/// the bare range and divides further where its own node joins at the
/// same level, so the division reads the text this pass leaves rather
/// than the text it was handed. A boolean chain joining at the other
/// operator stays whole on its row, the rows around it reading as one
/// run at one operator. `None` for an expression that is no operator
/// chain.
pub(super) fn operands(source: &Source, expr: &Expr, sheds: Sheds) -> Option<Vec<Operand>> {
    let mut out = Vec::new();
    match expr {
        Expr::BinOp(binary) => {
            let walk = Division::at(OperatorPrecedence::from(binary.op), sheds, source);
            walk.push(&binary.left, expr.into(), None, &mut out);
            walk.push(
                &binary.right,
                expr.into(),
                Some(binary.op.as_str()),
                &mut out,
            );
        }
        Expr::BoolOp(boolean) => {
            let walk = Division::at(OperatorPrecedence::from(boolean.op), sheds, source);
            for (index, value) in boolean.values.iter().enumerate() {
                let lead = (index > 0).then(|| boolean.op.as_str());
                walk.push(value, expr.into(), lead, &mut out);
            }
        }
        Expr::Compare(compare) => {
            let walk = Division::at(OperatorPrecedence::from(expr), sheds, source);
            let held = |operand: &Expr| walk.operand_range(operand, expr.into());
            out.push(Operand {
                lead: None,
                range: held(&compare.left),
            });
            for (operator, operand) in compare.ops.iter().zip(&compare.comparators) {
                out.push(Operand {
                    lead: Some(operator.as_str()),
                    range: held(operand),
                });
            }
        }
        _ => return None,
    }
    Some(out)
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
        operands(&source, expr, &holds).map(|found| {
            found
                .iter()
                .map(|o| source.slice(o.range).to_owned())
                .collect()
        })
    }

    /// `src`'s chain divided with every grouping pair shedding.
    fn divided_shedding(src: &str) -> Option<Vec<String>> {
        let source = parse(src);
        let expr = first_expr(&source);
        let all = |_: TextRange| true;
        operands(&source, expr, &all).map(|found| {
            found
                .iter()
                .map(|o| source.slice(o.range).to_owned())
                .collect()
        })
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
    #[case("a and b or c and d", &["a and b", "c and d"])]
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
    #[case("a + (b + c) + d", &["a", "b", "c", "d"])]
    #[case("a and (b == 1)", &["a", "b == 1"])]
    fn operands_divide_through_a_pair_the_pass_sheds(#[case] src: &str, #[case] expected: &[&str]) {
        let divided = divided_shedding(src).expect("the source holds a chain");
        assert_eq!(
            divided.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }

    #[rstest]
    #[case("a or (b and c)", &["a", "b and c"])]
    #[case("a and (b or c) and d", &["a", "b or c", "d"])]
    fn operands_hold_a_chain_binding_at_the_other_boolean_operator(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let divided = divided_shedding(src).expect("the source holds a chain");
        assert_eq!(
            divided.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }
}

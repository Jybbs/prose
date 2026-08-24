//! How constant an operand reads, the ladder deciding which side of a
//! test leads.

use ruff_python_ast::{Expr, UnaryOp};
use ruff_python_stdlib::str::is_cased_uppercase;

/// How constant an operand reads, the ladder deciding which side of a
/// test leads. The variants rank by declaration order, so a literal
/// outranks a `SCREAMING_CASE` name and both outrank everything else.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Constancy {
    Unlikely,
    Probably,
    Definitely,
}

/// Scores `expr` on the constancy ladder. A literal is `Definitely`, a
/// `SCREAMING_CASE` name or attribute is `Probably`, a collection or
/// arithmetic expression takes the weakest score among its parts, and
/// everything else is `Unlikely`.
pub(super) fn constancy(expr: &Expr) -> Constancy {
    match expr {
        _ if expr.is_literal_expr() => Constancy::Definitely,
        Expr::Attribute(attribute) => named_constancy(&attribute.attr),
        Expr::BinOp(bin_op) => constancy(&bin_op.left).min(constancy(&bin_op.right)),
        Expr::Dict(dict) => weakest(dict.iter_values().chain(dict.iter_keys().flatten())),
        Expr::List(list) => weakest(list),
        Expr::Name(name) => named_constancy(&name.id),
        Expr::Tuple(tuple) => weakest(tuple),
        Expr::UnaryOp(unary)
            if matches!(unary.op, UnaryOp::Invert | UnaryOp::UAdd | UnaryOp::USub) =>
        {
            constancy(&unary.operand)
        }
        _ => Constancy::Unlikely,
    }
}

/// The constancy an identifier carries, `Probably` for a
/// `SCREAMING_CASE` name and `Unlikely` otherwise.
fn named_constancy(identifier: &str) -> Constancy {
    if is_cased_uppercase(identifier) {
        Constancy::Probably
    } else {
        Constancy::Unlikely
    }
}

/// The weakest constancy among `exprs`, `Definitely` for an empty
/// collection.
fn weakest<'a>(exprs: impl IntoIterator<Item = &'a Expr>) -> Constancy {
    exprs
        .into_iter()
        .map(constancy)
        .min()
        .unwrap_or(Constancy::Definitely)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

    fn constancy_of(src: &str) -> Constancy {
        let source = parse(src);
        constancy(first_expr(&source))
    }

    #[rstest]
    #[case("[0, 1]", Constancy::Definitely)]
    #[case("[]", Constancy::Definitely)]
    #[case("[a, 1]", Constancy::Unlikely)]
    #[case("(1, 2)", Constancy::Definitely)]
    #[case("(1, LIMIT)", Constancy::Probably)]
    #[case("{'k': 1}", Constancy::Definitely)]
    #[case("{'k': v}", Constancy::Unlikely)]
    #[case("{LIMIT: 1}", Constancy::Probably)]
    #[case("{**base, 'k': 1}", Constancy::Unlikely)]
    #[case("{**BASE, 'k': 1}", Constancy::Probably)]
    fn constancy_takes_a_collection_at_its_weakest_part(
        #[case] src: &str,
        #[case] expected: Constancy,
    ) {
        assert_eq!(constancy_of(src), expected);
    }

    #[rstest]
    #[case("LIMIT", Constancy::Probably)]
    #[case("limit", Constancy::Unlikely)]
    #[case("cfg.LIMIT", Constancy::Probably)]
    #[case("cfg.limit", Constancy::Unlikely)]
    fn constancy_reads_a_name_by_its_casing(#[case] src: &str, #[case] expected: Constancy) {
        assert_eq!(constancy_of(src), expected);
    }

    #[rstest]
    #[case("42", Constancy::Definitely)]
    #[case("-42", Constancy::Definitely)]
    #[case("+42", Constancy::Definitely)]
    #[case("~MASK", Constancy::Probably)]
    #[case("LIMIT + 1", Constancy::Probably)]
    #[case("limit + 1", Constancy::Unlikely)]
    #[case("f()", Constancy::Unlikely)]
    #[case("not flag", Constancy::Unlikely)]
    fn constancy_scores_a_literal_above_a_computed_operand(
        #[case] src: &str,
        #[case] expected: Constancy,
    ) {
        assert_eq!(constancy_of(src), expected);
    }
}

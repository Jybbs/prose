//! The bracket a layout rule breaks a value open at once its row
//! crosses the cap, and the width of the opening row that expansion
//! leaves.

use ruff_python_ast::Expr;
use ruff_text_size::{Ranged, TextSize};

use crate::{primitives::slots::holds_exactly, source::Source};

/// Returns the display width from `start` through the bracket a layout
/// rule breaks `expr` open at, meaning the `(` of a call
/// [`Source::explodable_arguments`] lists while `explodes_calls` holds,
/// or the opener of a literal [`Source::expandable_literals`] lists.
/// `None` for any other expression and for a bracket on a later row
/// than `start`.
pub(crate) fn opener_width(
    source: &Source,
    expr: &Expr,
    start: TextSize,
    explodes_calls: bool,
) -> Option<usize> {
    let opener = match expr {
        Expr::Call(call) => {
            let arguments = call.arguments.range();
            (explodes_calls && holds_exactly(source.explodable_arguments(), arguments))
                .then_some(arguments.start())?
        }
        _ => holds_exactly(source.expandable_literals(), expr.range()).then_some(expr.start())?,
    };
    let end = opener + TextSize::of('(');
    source
        .same_line(start, end)
        .then(|| source.width_between(start, end))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    #[rstest]
    #[case::call_arguments("x = frobnicate(a, b)\n", Some(11))]
    #[case::chained_call_takes_its_own_list("x = a.b(c).d(e)\n", Some(9))]
    #[case::list_literal("x = [a, b]\n", Some(1))]
    #[case::one_entry_dict("x = {a: b}\n", Some(1))]
    #[case::call_without_arguments("x = frobnicate()\n", None)]
    #[case::single_element_list("x = [a]\n", None)]
    #[case::commented_arguments("x = f(\n    a,  # note\n    b,\n)\n", None)]
    #[case::opener_on_a_later_row("x = (\n    f\n)(a)\n", None)]
    #[case::name("x = value\n", None)]
    fn opener_width_reaches_the_bracket_a_layout_rule_opens(
        #[case] src: &str,
        #[case] expected: Option<usize>,
    ) {
        let source = parse(src);
        let value = first_value(&source);
        assert_eq!(opener_width(&source, value, value.start(), true), expected);
    }

    #[rstest]
    #[case::call_arguments("x = frobnicate(a, b)\n", None)]
    #[case::list_literal("x = [a, b]\n", Some(1))]
    fn opener_width_leaves_calls_out_where_none_explode(
        #[case] src: &str,
        #[case] expected: Option<usize>,
    ) {
        let source = parse(src);
        let value = first_value(&source);
        assert_eq!(opener_width(&source, value, value.start(), false), expected);
    }
}

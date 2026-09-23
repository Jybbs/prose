//! The bracket a layout rule breaks a value open at once its row
//! crosses the cap, and the width of the opening row that expansion
//! leaves.

use ruff_python_ast::Expr;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{primitives::slots::item_holding, source::Source};

/// The display width from `start` through the bracket `expr` opens
/// across rows at once a layout rule expands it, the `(` of a call
/// `reflow-calls` explodes or a literal's own opener where
/// `reflow-collections` expands it, where that bracket sits on
/// `start`'s row. `None` for a call without arguments or with a comment
/// inside them, a literal [`Source::expandable_literals`] leaves out,
/// and every other expression.
pub(crate) fn opener_width(source: &Source, expr: &Expr, start: TextSize) -> Option<usize> {
    let listed = |ranges: &[TextRange], range: TextRange| {
        item_holding(ranges, range.start()).is_some_and(|held| *held == range)
    };
    let opener = match expr {
        Expr::Call(call) if listed(source.explodable_arguments(), call.arguments.range()) => {
            call.arguments.start()
        }
        _ if listed(source.expandable_literals(), expr.range()) => expr.start(),
        _ => return None,
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
        assert_eq!(opener_width(&source, value, value.start()), expected);
    }
}

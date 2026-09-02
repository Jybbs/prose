//! Shared layout helpers for laying a construct out across lines,
//! covering one-per-line expansion, greedy line filling, and reading
//! the bracket shape a block already carries.

mod pack;
mod shapes;

use pack::flush_bracket_open;
pub(crate) use pack::{explode_parens, item_indent, pack, requires_expand};
pub(crate) use shapes::{
    is_collapse_only, is_collapsible, is_column_shaped, is_fractured, is_layoutable, is_multi_entry,
};

use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_text_size::TextRange;

use crate::{primitives::INDENT_STEP, source::Source};

/// What [`explode_parens`] writes after each exploded item.
#[derive(Clone, Copy)]
pub(crate) enum Separator {
    /// A `,` between items with none after the last.
    Comma,
    /// A `,` between items and after the last.
    CommaTrailing,
    /// Nothing between items, the shape adjacent string literals take.
    None,
}

impl Separator {
    /// The comma form, trailing or not.
    pub(crate) fn comma(trailing: bool) -> Self {
        if trailing {
            Self::CommaTrailing
        } else {
            Self::Comma
        }
    }

    /// The text following the item at `index` of `count`.
    fn after(self, index: usize, count: usize) -> &'static str {
        match self {
            Self::Comma if index + 1 < count => ",",
            Self::CommaTrailing => ",",
            Self::Comma | Self::None => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

    #[rstest]
    #[case("[a]", true)]
    #[case("{b}", true)]
    #[case("(c, d)", true)]
    #[case("{e: f}", true)]
    #[case("g[h]", true)]
    #[case("[x for x in y]", true)]
    #[case("{x for x in y}", true)]
    #[case("{k: v for k, v in y}", true)]
    #[case("(x for x in y)", true)]
    #[case("plain", false)]
    #[case("a + b", false)]
    fn is_collapsible_covers_literals_subscripts_and_comprehensions(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(is_collapsible(expr), expected);
    }

    #[rstest]
    #[case("[\n    1,\n    2,\n]", true)]
    #[case("{\n    'a': 1\n}", true)]
    #[case("(\n    'only',\n)", true)]
    #[case("[\r\n    1,\r\n]", true)]
    #[case("[\r    1,\r]", true)]
    #[case("(\n    alpha,\n    beta,\n)", true)]
    #[case("[1,\n 2]", false)]
    #[case("[1,\r 2]", false)]
    #[case("(\n    value,)", false)]
    #[case("(1,\n    2,\n    3)", false)]
    #[case("{\n}", false)]
    #[case("[1, 2]", false)]
    fn is_column_shaped_requires_both_brackets_alone_on_their_lines(
        #[case] slice: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_column_shaped(slice), expected);
    }

    #[rstest]
    #[case("[a, b]", true)]
    #[case("{a: 1, b: 2}", true)]
    #[case("(a, b)", true)]
    #[case("{a: 1}", false)]
    #[case("[a]", false)]
    #[case("()", false)]
    #[case("a, b", false)]
    fn is_multi_entry_requires_two_bracketed_entries(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(is_multi_entry(expr.into()), expected);
    }

    #[test]
    fn pack_carries_a_lone_overflowing_item_onto_its_own_line() {
        // prefix 10, budget 14, item widths 8/8: neither pairs onto a
        // line, so each forced item lands alone despite overflowing.
        assert_eq!(pack(&[8, 8], 10, 2, 14), vec![0..1, 1..2]);
    }

    #[test]
    fn pack_fills_each_line_before_opening_the_next() {
        // prefix 5, budget 16: 4 then 4 (5+4=9, +2+4=15) fit, 4 more
        // (15+2+4=21) overflows and opens a line that then takes the 4.
        assert_eq!(pack(&[4, 4, 4], 5, 2, 16), vec![0..2, 2..3]);
    }

    #[test]
    fn pack_joins_items_edge_to_edge_under_a_zero_separator() {
        // budget 10, no separator: 4+4 fills to 8, the third 4 opens a
        // line, the shape adjacent string literals take.
        assert_eq!(pack(&[4, 4, 4], 0, 0, 10), vec![0..2, 2..3]);
    }

    #[test]
    fn pack_keeps_one_line_when_every_item_fits() {
        assert_eq!(pack(&[1, 1, 1], 5, 2, 80), vec![0..3]);
    }

    #[rstest]
    #[case("(a, b)", true)]
    #[case("(a,)", false)]
    #[case("()", false)]
    #[case("a, b, c", false)]
    #[case("(a + b)", false)]
    #[case("[a, b]", true)]
    fn requires_expand_gates_parenthesized_multi_item_tuples(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(requires_expand(expr), expected);
    }
}

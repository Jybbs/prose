//! Shared layout helpers for laying a construct out across lines,
//! covering one-per-line expansion, greedy line filling, and reading
//! the bracket shape a block already carries.

use std::ops::Range;

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

/// Builds the one-per-line expansion `(\n<prefix>item<sep>\n…\n<indent>)`
/// for `count` items at `indent`. `render` writes item `i` into the
/// buffer, and `separator` writes `<sep>`. Items sit at [`item_indent`],
/// the closing `)` at `indent`.
pub(crate) fn explode_parens(
    newline: &str,
    indent: usize,
    count: usize,
    mut render: impl FnMut(&mut String, usize),
    separator: Separator,
) -> String {
    let prefix = " ".repeat(item_indent(indent));
    let mut out = String::from("(");
    for i in 0..count {
        out.push_str(newline);
        out.push_str(&prefix);
        render(&mut out, i);
        out.push_str(separator.after(i, count));
    }
    out.push_str(newline);
    out.push_str(&prefix[..indent]);
    out.push(')');
    out
}

/// True for the collapse-only forms, a subscript whose `[index]` joins
/// onto one line whatever the index shape and the four comprehensions,
/// each joining when it fits and never expanding the way a literal does.
pub(crate) fn is_collapse_only(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::DictComp(_)
            | Expr::Generator(_)
            | Expr::ListComp(_)
            | Expr::SetComp(_)
            | Expr::Subscript(_)
    )
}

/// True for the bracketed expressions the visitor measures for a
/// single-line collapse: the four collection literals plus the
/// collapse-only forms, a subscript and the four comprehensions.
pub(crate) fn is_collapsible(expr: &Expr) -> bool {
    is_layoutable(expr) || is_collapse_only(expr)
}

/// True when `slice`, a bracketed construct's source text, already
/// carries the flush column shape the expand path emits, its opening
/// bracket ending its line and its closing bracket opening its own.
/// Every other break is a fracture.
pub(crate) fn is_column_shaped(slice: &str) -> bool {
    flush_bracket_open(slice).is_some_and(|(_, body)| {
        body.rsplit_once('\n')
            .is_some_and(|(_, close)| close.trim_start().len() == 1)
    })
}

/// True when `range` carries a break a join could close, spanning
/// lines without already holding the flush column shape.
pub(crate) fn is_fractured(source: &Source, range: TextRange) -> bool {
    source.contains_line_break(range) && !is_column_shaped(source.slice(range))
}

/// True for the four collection-literal `Expr` variants the layout
/// rules lay out, `Dict`, `List`, `Set`, and `Tuple`.
pub(crate) fn is_layoutable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Dict(_) | Expr::List(_) | Expr::Set(_) | Expr::Tuple(_)
    )
}

/// True for a `Dict`, `List`, `Set`, or parenthesized `Tuple` node
/// carrying more than one entry. A bare tuple carries no bracket pair
/// to hang broken lines on.
pub(crate) fn is_multi_entry(node: AnyNodeRef) -> bool {
    match node {
        AnyNodeRef::ExprDict(dict) => dict.len() > 1,
        AnyNodeRef::ExprList(list) => list.len() > 1,
        AnyNodeRef::ExprSet(set) => set.len() > 1,
        AnyNodeRef::ExprTuple(tuple) => tuple.parenthesized && tuple.len() > 1,
        _ => false,
    }
}

/// The column an exploded construct opens its items at, one
/// `INDENT_STEP` past the `indent` its closing bracket lands on.
pub(crate) fn item_indent(indent: usize) -> usize {
    indent + INDENT_STEP
}

/// Greedily groups item indices into lines, each opening after
/// `prefix_width` and packing items joined by `separator_width` columns
/// up to `budget`. The first item on every line is always placed, so an
/// item whose own line overflows still lands rather than splitting away.
pub(crate) fn pack(
    widths: &[usize],
    prefix_width: usize,
    separator_width: usize,
    budget: usize,
) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut line_width = 0;
    for (i, &width) in widths.iter().enumerate() {
        if i == start {
            line_width = prefix_width + width;
        } else if line_width + separator_width + width <= budget {
            line_width += separator_width + width;
        } else {
            lines.push(start..i);
            start = i;
            line_width = prefix_width + width;
        }
    }
    lines.push(start..widths.len());
    lines
}

/// True for a `Dict`, `List`, `Set`, or parenthesized `Tuple` shape
/// the expand path canonicalizes, the [`is_multi_entry`] shapes and
/// the one-entry `Dict`. An empty or single-item collection otherwise
/// has nothing to flow.
pub(crate) fn requires_expand(expr: &Expr) -> bool {
    is_multi_entry(expr.into()) || expr.as_dict_expr().is_some_and(|dict| dict.len() == 1)
}

/// Splits `block` at its first line break when that opening line holds
/// its bracket alone, yielding the bracket and the body beneath. A
/// single-line block and one whose first line carries content beside
/// the bracket both return `None`.
fn flush_bracket_open(block: &str) -> Option<(&str, &str)> {
    let (open, body) = block.split_once('\n')?;
    (open.trim().len() == 1).then_some((open, body))
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
    #[case("(\n    alpha,\n    beta,\n)", true)]
    #[case("[1,\n 2]", false)]
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

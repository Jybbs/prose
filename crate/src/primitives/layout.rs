//! Shared layout helpers for laying a construct out across lines,
//! covering one-per-line expansion, greedy line filling, reading the
//! bracket shape a block already carries, and re-indenting an
//! already-exploded block.

use std::{borrow::Cow, ops::Range};

use ruff_python_ast::Expr;
use ruff_python_trivia::textwrap::{dedent, indent};

use crate::primitives::{INDENT_STEP, inline::indent_width};

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

/// Splits `block` at its first line break when that opening line holds
/// its bracket alone, yielding the bracket and the body beneath. A
/// single-line block and one whose first line carries content beside
/// the bracket both return `None`.
pub(crate) fn flush_bracket_open(block: &str) -> Option<(&str, &str)> {
    let (open, body) = block.split_once('\n')?;
    (open.trim().len() == 1).then_some((open, body))
}

/// True for the four collection-literal `Expr` variants the layout
/// rules lay out, `Dict`, `List`, `Set`, and `Tuple`.
pub(crate) fn is_layoutable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Dict(_) | Expr::List(_) | Expr::Set(_) | Expr::Tuple(_)
    )
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

/// Re-indents the multi-line bracket `block` that `explode_parens` or
/// collection layout emits at one indent so its closing bracket lands
/// at `to`, keeping the body's relative depth. The opening line stays
/// flush, since the caller places it inline after the keyword. The body
/// dedents to its least-indented line, the closing bracket, then
/// re-indents to `to`. Only the exploded form re-indents, so a
/// single-line `block` and one whose opening bracket shares its first
/// line with content both return borrowed. A caller excludes a block
/// whose interior spans a string literal, whose lines `indent` would pad.
pub(crate) fn reindent_block(block: &str, to: usize) -> Cow<'_, str> {
    let Some((open, body)) = flush_bracket_open(block) else {
        return Cow::Borrowed(block);
    };
    Cow::Owned(format!(
        "{open}\n{}",
        indent(&dedent(body), &" ".repeat(to))
    ))
}

/// The columns [`reindent_block`] moves `block`'s body by when it
/// re-indents to `to`, zero where it leaves the block borrowed.
pub(crate) fn reindent_shift(block: &str, to: usize) -> isize {
    match reindent_block(block, to) {
        Cow::Borrowed(_) => 0,
        Cow::Owned(moved) => closing_indent(&moved) as isize - closing_indent(block) as isize,
    }
}

/// The indent width of `block`'s last line.
fn closing_indent(block: &str) -> usize {
    indent_width(block.rsplit_once('\n').map_or(block, |(_, last)| last))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

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

    #[test]
    fn reindent_block_borrows_a_packed_first_line() {
        // Content beside the opening bracket marks a packed block, not
        // the exploded form, so it holds its source shape.
        assert_matches!(
            reindent_block("(a, b,\n    c)", 8),
            Cow::Borrowed("(a, b,\n    c)")
        );
    }

    #[test]
    fn reindent_block_borrows_a_single_line_block() {
        assert_matches!(reindent_block("{a: b}", 4), Cow::Borrowed("{a: b}"));
    }

    #[rstest]
    #[case("{\n    a,\n    b,\n}", 4, "{\n        a,\n        b,\n    }")]
    #[case("{\n        a,\n    }", 0, "{\n    a,\n}")]
    #[case("{\n        a,\n    }", 4, "{\n        a,\n    }")]
    #[case(
        "[\n    a,\n    [\n        b,\n    ],\n]",
        4,
        "[\n        a,\n        [\n            b,\n        ],\n    ]"
    )]
    #[case("{\n    a,\n\n    b,\n}", 4, "{\n        a,\n\n        b,\n    }")]
    fn reindent_block_shifts_body_to_target_keeping_relative_depth(
        #[case] block: &str,
        #[case] to: usize,
        #[case] expected: &str,
    ) {
        assert_eq!(reindent_block(block, to), expected);
    }

    #[rstest]
    #[case("(a, b,\n    c)", 8, 0)]
    #[case("{a: b}", 4, 0)]
    #[case("{\n    a,\n    b,\n}", 4, 4)]
    #[case("{\n        a,\n    }", 0, -4)]
    fn reindent_shift_reports_the_columns_the_body_moves(
        #[case] block: &str,
        #[case] to: usize,
        #[case] expected: isize,
    ) {
        assert_eq!(reindent_shift(block, to), expected);
    }
}

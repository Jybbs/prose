//! Shared layout helpers for laying a construct out across lines,
//! covering one-per-line expansion, greedy line filling, reading the
//! bracket shape a block already carries, measuring where a block's
//! continuation lines hang, and moving them to a new column.

use std::{borrow::Cow, ops::Range};

use ruff_python_ast::{Expr, StringLike, helpers::any_over_expr};
use ruff_python_trivia::textwrap::{dedent, indent};
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{TextRange, TextSize};

use crate::{
    primitives::{INDENT_STEP, inline::indent_width},
    source::Source,
};

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

/// True where `block`, the text of one construct, hangs from the row
/// `start` opens on rather than from a column inside that row, so the
/// whole block travels with the row. A run aligned under an interior
/// bracket would land against nothing once the row moves.
pub(crate) fn hangs_from_its_row(source: &Source, start: TextSize, block: &str) -> bool {
    continuation_indent(block).is_some_and(|body| body <= source.line_indent_width(start))
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

/// `range`'s source text with its continuation lines moved so it hangs
/// from `indent`, and borrowed where it holds no break, hangs from a
/// column inside its own row, or carries a string part spanning rows
/// whose interior the move would pad. `strings` names the expressions
/// whose parts the move must leave alone.
pub(crate) fn placed_block<'s, 'e>(
    source: &'s Source,
    range: TextRange,
    strings: impl IntoIterator<Item = &'e Expr>,
    indent: usize,
) -> Cow<'s, str> {
    let block = source.slice(range);
    if hangs_from_its_row(source, range.start(), block)
        && !strings
            .into_iter()
            .any(|expr| spans_a_string_part(source, expr))
    {
        return reindent_continuation(block, indent);
    }
    Cow::Borrowed(block)
}

/// Re-indents `block`'s continuation lines so its least-indented line
/// lands at `to`, keeping the body's relative depth. The opening line
/// stays as written, since the caller places it inline after whatever
/// precedes it. A single-line `block` returns borrowed. A caller
/// screens the block through [`spans_a_string_part`] first, whose
/// interior `to` would otherwise pad.
pub(crate) fn reindent_continuation(block: &str, to: usize) -> Cow<'_, str> {
    let Some((open, body)) = block.split_once('\n') else {
        return Cow::Borrowed(block);
    };
    Cow::Owned(format!(
        "{open}\n{}",
        indent(&dedent(body), &" ".repeat(to))
    ))
}

/// The columns [`reindent_continuation`] moves `block`'s body by when it
/// re-indents to `to`, zero where it leaves the block borrowed.
pub(crate) fn reindent_shift(block: &str, to: usize) -> isize {
    continuation_indent(block).map_or(0, |body| to.cast_signed() - body.cast_signed())
}

/// True where a string part inside `expr` itself spans rows, whose
/// interior a re-indent would pad. A stacked run of single-line parts
/// carries its break between parts and moves whole, so it reads false.
pub(crate) fn spans_a_string_part(source: &Source, expr: &Expr) -> bool {
    any_over_expr(expr, |e| {
        StringLike::try_from(e)
            .is_ok_and(|run| run.parts().any(|part| source.contains_line_break(part)))
    })
}

/// The least indent among `block`'s continuation lines, blank lines
/// skipped, and `None` where `block` carries no continuation. The value
/// is the column [`reindent_continuation`] moves to `to`, so the lines
/// are counted the way the `dedent` behind that move counts them.
fn continuation_indent(block: &str) -> Option<usize> {
    let (_, body) = block.split_once('\n')?;
    body.universal_newlines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| indent_width(&line))
        .min()
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
    use std::assert_matches;

    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::{first_expr, parse};

    #[rstest]
    #[case("[a, b]", None)]
    #[case("{\n    a,\n}", Some(0))]
    #[case("helper(a,\n       b)", Some(7))]
    #[case("{\n        a,\n    }", Some(4))]
    #[case("{\n    a,\n\n    b,\n}", Some(0))]
    #[case("\"aaa\"\n    \"bbb\"", Some(4))]
    #[case("{\r\n    a,\r\n}", Some(0))]
    #[case("{\n        a,\n    \n}", Some(0))]
    fn continuation_indent_reads_the_shallowest_continuation_line(
        #[case] block: &str,
        #[case] expected: Option<usize>,
    ) {
        assert_eq!(continuation_indent(block), expected);
    }

    #[rstest]
    #[case("f(\n    a,\n)", true)]
    #[case("[\n    a,\n]", true)]
    #[case("f(a,\n  b)", false)]
    #[case("f(a)", false)]
    fn hangs_from_its_row_rejects_a_run_under_an_interior_bracket(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(hangs_from_its_row(&source, expr.start(), src), expected);
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
    fn reindent_continuation_borrows_a_single_line_block() {
        assert_matches!(reindent_continuation("{a: b}", 4), Cow::Borrowed("{a: b}"));
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
    #[case(
        "helper(\n    b,\n    c\n)",
        4,
        "helper(\n        b,\n        c\n    )"
    )]
    #[case("\"aaa\"\n\"bbb\"", 4, "\"aaa\"\n    \"bbb\"")]
    #[case("(a, b,\n    c)", 8, "(a, b,\n        c)")]
    fn reindent_continuation_shifts_body_to_target_keeping_relative_depth(
        #[case] block: &str,
        #[case] to: usize,
        #[case] expected: &str,
    ) {
        assert_eq!(reindent_continuation(block, to), expected);
    }

    #[rstest]
    #[case("(a, b,\n    c)", 8, 4)]
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

    #[rstest]
    #[case("\"\"\"line1\nline2\"\"\"", true)]
    #[case("[\n    a,\n    \"\"\"line1\nline2\"\"\",\n]", true)]
    #[case("f\"\"\"head\nline2 {value}\"\"\"", true)]
    #[case("(\"aaa\"\n\"bbb\")", false)]
    #[case("[\n    a,\n    b,\n]", false)]
    #[case("helper(\n    a,\n    b\n)", false)]
    #[case("[a, b]", false)]
    fn spans_a_string_part_reads_each_part_rather_than_the_run(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(spans_a_string_part(&source, expr), expected);
    }
}

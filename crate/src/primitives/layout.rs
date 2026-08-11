//! Shared layout helpers for laying a construct out across lines,
//! covering one-per-line expansion, greedy line filling, reading the
//! bracket shape a block already carries, and moving a block's
//! continuation rows to a new column.

use std::{borrow::Cow, ops::Range};

use ruff_python_ast::{Expr, StringLike, helpers::any_over_expr, token::TokenKind};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextRange, TextSize};

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

/// True for the four collection-literal `Expr` variants the layout
/// rules lay out, `Dict`, `List`, `Set`, and `Tuple`.
pub(crate) fn is_layoutable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Dict(_) | Expr::List(_) | Expr::Set(_) | Expr::Tuple(_)
    )
}

/// True for a literal carrying more than one entry, `requires_expand`
/// apart from the one-entry `Dict`.
pub(crate) fn is_multi_entry(expr: &Expr) -> bool {
    requires_expand(expr) && expr.as_dict_expr().is_none_or(|dict| dict.len() > 1)
}

/// True for a `Dict`, `List`, `Set`, or parenthesized `Tuple` shape
/// the expand path canonicalizes. Multi-item `List`, `Set`, and
/// parenthesized `Tuple` qualify, as does any non-empty `Dict`. A bare
/// tuple carries no bracket pair to hang broken lines on, and an empty
/// or single-item collection has nothing to flow.
pub(crate) fn requires_expand(expr: &Expr) -> bool {
    match expr {
        Expr::Dict(d) => !d.is_empty(),
        Expr::List(l) => l.len() > 1,
        Expr::Set(s) => s.len() > 1,
        Expr::Tuple(t) => t.parenthesized && t.len() > 1,
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

/// Where a block lands: the indent the row carrying it lands at, the
/// column the block's own start lands at, and the offset of the item
/// opening that row. A block that is its own item takes
/// [`Landing::own_row`].
#[derive(Clone, Copy)]
pub(crate) struct Landing {
    pub(crate) column: usize,
    pub(crate) indent: usize,
    pub(crate) item: TextSize,
}

impl Landing {
    /// The landing of a block that opens its own row at `indent`,
    /// carrying no text ahead of `start` on that row.
    pub(crate) fn own_row(start: TextSize, indent: usize) -> Self {
        Self {
            column: indent,
            indent,
            item: start,
        }
    }
}

/// How a relocated block's rows move, `rows` carrying every movable
/// continuation row and `closer` seating the block's final movable row
/// on one column outright.
#[derive(Clone, Copy)]
pub(crate) struct Travel {
    pub(crate) rows: isize,
    closer: Option<usize>,
}

impl Travel {
    /// The move carrying every continuation row by `rows`.
    fn rigid(rows: isize) -> Self {
        Self { rows, closer: None }
    }

    /// True where every row stays where it was written.
    fn is_still(self) -> bool {
        self.rows == 0 && self.closer.is_none()
    }

    /// The indent the row at `lead` lands on, `is_last` marking the
    /// block's final movable row.
    fn placed(self, lead: usize, is_last: bool) -> usize {
        match self.closer {
            Some(column) if is_last => column,
            _ => lead.saturating_add_signed(self.rows),
        }
    }
}

/// The move `block`'s continuation rows make when it lands per
/// `landing`, its own start sitting at `start` in the source and its
/// frozen rows flagged in `frozen`. `None` where every continuation row
/// is blank or frozen.
///
/// A block whose movable rows sit at or left of the column its item
/// opens at hangs from its own row, so it rebases through
/// [`hanging_travel`] or, where that reads no bracket to hang inside,
/// rebases its shallowest row onto the landing indent. One whose rows
/// sit right of that column is aligned under a bracket inside its
/// opening row, so the whole block shifts by however far its own start
/// moved and the alignment survives. Reading the test against the
/// item's column rather than the block's keeps the answer the same once
/// an alignment pads the text ahead of the block, which moves the block
/// alone.
pub(crate) fn block_shift(
    source: &Source,
    block: &str,
    frozen: &[bool],
    start: TextSize,
    landing: Landing,
) -> Option<Travel> {
    let floor = movable_floor(block, frozen)?;
    if floor > source.column_of(landing.item) {
        return Some(Travel::rigid(
            landing.column.cast_signed() - source.column_of(start).cast_signed(),
        ));
    }
    Some(
        hanging_travel(block, frozen, landing)
            .unwrap_or_else(|| Travel::rigid(landing.indent.cast_signed() - floor.cast_signed())),
    )
}

/// `range`'s source text placed per `landing`, its continuation rows
/// travelling and every row a row-spanning string part freezes left
/// where the source wrote it. Borrowed where the block holds no movable
/// continuation row or already sits where it lands.
pub(crate) fn placed_block(source: &Source, range: TextRange, landing: Landing) -> Cow<'_, str> {
    let block = source.slice(range);
    let frozen = frozen_rows(source, range);
    match block_shift(source, block, &frozen, range.start(), landing) {
        Some(travel) if !travel.is_still() => Cow::Owned(shifted_rows(block, travel, &frozen)),
        _ => Cow::Borrowed(block),
    }
}

/// `block`'s continuation rows moved per `travel`, every blank row
/// passing through as written and the block borrowed where no row
/// moves. A caller screens the block through [`spans_a_string_part`]
/// first, whose interior a move would pad.
pub(crate) fn shifted_block(block: &str, travel: Travel) -> Cow<'_, str> {
    if travel.is_still() {
        return Cow::Borrowed(block);
    }
    Cow::Owned(shifted_rows(block, travel, &[]))
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

/// One flag per row of the block at `range`, set for every row opening
/// strictly inside a string token that itself spans rows. Shifting such
/// a row would pad the string's own interior, so a move holds it.
fn frozen_rows(source: &Source, range: TextRange) -> Vec<bool> {
    let rows = source.line_index(range.end()).get() - source.line_index(range.start()).get() + 1;
    let mut frozen = vec![false; rows];
    let tokens = source.tokens();
    let first = tokens
        .binary_search_by_start(range.start())
        .unwrap_or_else(|slot| slot.saturating_sub(1));
    let head = source.line_index(range.start()).get();
    for token in tokens[first..]
        .iter()
        .take_while(|t| t.start() < range.end())
    {
        if !matches!(
            token.kind(),
            TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle
        ) || !source.contains_line_break(token.range())
        {
            continue;
        }
        let opens = source.line_index(token.start()).get();
        let closes = source.line_index(token.end()).get();
        for row in (opens + 1)..=closes {
            if let Some(slot) = row.checked_sub(head).and_then(|r| frozen.get_mut(r)) {
                *slot = true;
            }
        }
    }
    frozen
}

/// The move for a block whose first row leaves a bracket open, seating
/// the shallowest interior row one [`INDENT_STEP`] inside
/// `landing.indent` and a closing row of the block's own back on that
/// indent, the shape [`explode_parens`] writes. A closing row the row
/// move already lands there travels with the rest. `None` where the
/// first row opens no bracket, where a row-spanning string opens on it,
/// or where an interior row itself opens with a closing bracket, whose
/// depth one move cannot follow.
fn hanging_travel(block: &str, frozen: &[bool], landing: Landing) -> Option<Travel> {
    let head = block.universal_newlines().next()?;
    if !head.trim_end().ends_with(['(', '[', '{']) || frozen.get(1).copied().unwrap_or(false) {
        return None;
    }
    let mut interior: Vec<(usize, bool)> = block
        .universal_newlines()
        .enumerate()
        .skip(1)
        .filter(|(row, line)| {
            !frozen.get(*row).copied().unwrap_or(false) && !line.trim().is_empty()
        })
        .map(|(_, line)| {
            (
                indent_width(&line),
                line.trim_start().starts_with([')', ']', '}']),
            )
        })
        .collect();
    let close = interior
        .last()
        .copied()
        .filter(|(_, closes)| *closes)
        .map(|(indent, _)| indent);
    if close.is_some() {
        interior.pop();
    }
    if interior.iter().any(|(_, closes)| *closes) {
        return None;
    }
    let floor = interior.iter().map(|(indent, _)| *indent).min()?;
    let rows = item_indent(landing.indent).cast_signed() - floor.cast_signed();
    Some(Travel {
        rows,
        closer: close
            .filter(|indent| indent.saturating_add_signed(rows) != landing.indent)
            .map(|_| landing.indent),
    })
}

/// The least indent among the movable non-blank continuation rows of
/// `block`, `None` where every continuation row is blank or frozen. An
/// empty `frozen` holds no row, the shape a caller passes when nothing
/// inside the block spans rows.
pub(crate) fn movable_floor(block: &str, frozen: &[bool]) -> Option<usize> {
    block
        .universal_newlines()
        .enumerate()
        .skip(1)
        .filter(|(row, line)| {
            !frozen.get(*row).copied().unwrap_or(false) && !line.trim().is_empty()
        })
        .map(|(_, line)| indent_width(&line))
        .min()
}

/// `block`'s continuation rows moved per `travel`, each blank row and
/// each row `frozen` marks passing through as written.
fn shifted_rows(block: &str, travel: Travel, frozen: &[bool]) -> String {
    let movable = |row: usize, line: &str| {
        row > 0 && !frozen.get(row).copied().unwrap_or(false) && !line.trim().is_empty()
    };
    let last = block
        .split_inclusive('\n')
        .enumerate()
        .filter(|(row, line)| movable(*row, line))
        .map(|(row, _)| row)
        .last();
    let mut out = String::with_capacity(block.len());
    for (row, line) in block.split_inclusive('\n').enumerate() {
        if !movable(row, line) {
            out.push_str(line);
            continue;
        }
        let lead = leading_indentation(line);
        let placed = travel.placed(lead.chars().count(), Some(row) == last);
        out.push_str(&" ".repeat(placed));
        out.push_str(&line[lead.len()..]);
    }
    out
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
    #[case("[a, b]", &[], None)]
    #[case("{\n    a,\n}", &[], Some(0))]
    #[case("helper(a,\n       b)", &[], Some(7))]
    #[case("{\n        a,\n    }", &[], Some(4))]
    #[case("{\n    a,\n\n    b,\n}", &[], Some(0))]
    #[case("\"aaa\"\n    \"bbb\"", &[], Some(4))]
    #[case("{\r\n    a,\r\n}", &[], Some(0))]
    #[case("{\n        a,\n    \n}", &[], Some(0))]
    #[case("f(\n  a,\n)", &[false, true, false], Some(0))]
    #[case("f(\n  a,\n)", &[false, true, true], None)]
    fn movable_floor_reads_the_shallowest_movable_continuation_row(
        #[case] block: &str,
        #[case] frozen: &[bool],
        #[case] expected: Option<usize>,
    ) {
        assert_eq!(movable_floor(block, frozen), expected);
    }

    #[rstest]
    #[case("(\n        a,\n        )", &[], Some((0, Some(4))))]
    #[case("(\n  a,\n  )", &[], Some((6, Some(4))))]
    #[case("[\n  a,\n\n  b,\n  ]", &[], Some((6, Some(4))))]
    #[case("(\n      a,\n  )", &[], Some((2, None)))]
    #[case("(\n  a,\n  b)", &[], Some((6, None)))]
    #[case("{\r\n  a,\r\n  }", &[], Some((6, Some(4))))]
    #[case("(a,\n  b)", &[], None)]
    #[case("plain", &[], None)]
    #[case("(\n)", &[], None)]
    #[case("f(\n  a,\n) + g(\n  b,\n)", &[], None)]
    #[case("(\n  a,\n  )", &[false, true, false], None)]
    fn hanging_travel_seats_an_open_bracket_one_step_in(
        #[case] block: &str,
        #[case] frozen: &[bool],
        #[case] expected: Option<(isize, Option<usize>)>,
    ) {
        let landing = Landing::own_row(TextSize::new(0), 4);
        assert_eq!(
            hanging_travel(block, frozen, landing).map(|travel| (travel.rows, travel.closer)),
            expected,
        );
    }

    #[rstest]
    #[case("f(a,\n  b)", 0, "f(a,\n  b)")]
    #[case("f(a,\n  b)", 3, "f(a,\n     b)")]
    #[case("f(a,\n    b)", -2, "f(a,\n  b)")]
    #[case("f(a,\n\n  b)", 2, "f(a,\n\n    b)")]
    #[case("single", 4, "single")]
    fn shifted_block_moves_every_non_blank_continuation_row(
        #[case] block: &str,
        #[case] shift: isize,
        #[case] expected: &str,
    ) {
        assert_eq!(shifted_block(block, Travel::rigid(shift)), expected);
    }

    #[rstest]
    #[case("f(\n  a,\n  )", "f(\n    a,\n  )")]
    #[case("f(\n  a,\n  )\n", "f(\n    a,\n  )\n")]
    #[case("f(\n  a,\n\n  )", "f(\n    a,\n\n  )")]
    #[case("f(\n  a,\n      )", "f(\n    a,\n  )")]
    fn a_closer_row_seats_on_its_own_column(#[case] block: &str, #[case] expected: &str) {
        let travel = Travel {
            rows: 2,
            closer: Some(2),
        };
        assert_eq!(shifted_block(block, travel), expected);
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
        assert_eq!(is_multi_entry(expr), expected);
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
}

//! Moves a relocated block's continuation rows to the column it lands
//! at. [`block_shift`] reads the move a block makes against its
//! [`Landing`], [`placed_block`] applies it to a source range, and a
//! row a row-spanning string freezes stays where the source wrote it.

use std::borrow::Cow;

use ruff_python_ast::{Expr, StringLike, helpers::any_over_expr, token::TokenKind};
use ruff_python_parser::{Mode, lexer::lex};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::{Line, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        inline::indent_width,
        layout::item_indent,
        tokens::{is_closer, is_opener},
    },
    source::Source,
};

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
/// alone. Rows sitting exactly one indent step past the row the block
/// opens on hang from that row whatever column the item opens at, and
/// re-seat one step past the landing indent.
pub(crate) fn block_shift(
    source: &Source,
    block: &str,
    frozen: &[bool],
    start: TextSize,
    landing: Landing,
) -> Option<Travel> {
    let floor = movable_floor(block, frozen)?;
    let past_item = floor > source.column_of(landing.item);
    if past_item && floor != item_indent(source.line_indent_width(start)) {
        return Some(Travel::rigid(
            landing.column.cast_signed() - source.column_of(start).cast_signed(),
        ));
    }
    let rebase = if past_item {
        item_indent(landing.indent)
    } else {
        landing.indent
    };
    Some(
        hanging_travel(block, frozen, landing)
            .unwrap_or_else(|| Travel::rigid(rebase.cast_signed() - floor.cast_signed())),
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
pub(crate) fn frozen_rows(source: &Source, range: TextRange) -> Vec<bool> {
    let rows = source.line_index(range.end()).get() - source.line_index(range.start()).get() + 1;
    let mut frozen = vec![false; rows];
    let head = source.line_index(range.start()).get();
    for token in source.tokens_overlapping(range) {
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
/// the shallowest interior row one `INDENT_STEP` inside `landing.indent`
/// and a closing row of the block's own back on that indent, the shape
/// `explode_parens` writes. A closing row the row move already lands
/// there travels with the rest. `None` where the first row opens no
/// bracket, where a row-spanning string opens on it, or where an
/// interior row itself opens with a closing bracket, whose depth one
/// move cannot follow, a last row opening with the closer of a bracket
/// an interior row opened reading as interior too.
fn hanging_travel(block: &str, frozen: &[bool], landing: Landing) -> Option<Travel> {
    let head = block.universal_newlines().next()?;
    if !head.trim_end().ends_with(['(', '[', '{']) || frozen.get(1) == Some(&true) {
        return None;
    }
    let rows: Vec<Line> = movable_rows(block, frozen).collect();
    let opens_with_closer = |line: &Line| line.trim_start().starts_with([')', ']', '}']);
    let close = rows
        .last()
        .filter(|line| opens_with_closer(line) && closes_the_head(line.trim_start()))
        .map(|line| indent_width(line));
    let interior = &rows[..rows.len() - usize::from(close.is_some())];
    if interior.iter().any(opens_with_closer) {
        return None;
    }
    let floor = interior.iter().map(|line| indent_width(line)).min()?;
    let rows = item_indent(landing.indent).cast_signed() - floor.cast_signed();
    Some(Travel {
        rows,
        closer: close
            .filter(|indent| indent.saturating_add_signed(rows) != landing.indent)
            .map(|_| landing.indent),
    })
}

/// True where the closer `row` opens with closes the bracket the block's
/// head left open rather than one an interior row opened, meaning no
/// closer later on the row is left unmatched by an opener ahead of it.
fn closes_the_head(row: &str) -> bool {
    let mut lexer = lex(row, Mode::Expression);
    let mut depth = 0_usize;
    // The leading closer is the one under test.
    lexer.next_token();
    loop {
        let kind = lexer.next_token();
        if kind == TokenKind::EndOfFile {
            return true;
        }
        if is_opener(kind) {
            depth += 1;
        } else if is_closer(kind) {
            let Some(shallower) = depth.checked_sub(1) else {
                return false;
            };
            depth = shallower;
        }
    }
}

/// The least indent among the movable non-blank continuation rows of
/// `block`, `None` where every continuation row is blank or frozen. An
/// empty `frozen` holds no row, the shape a caller passes when nothing
/// inside the block spans rows.
fn movable_floor(block: &str, frozen: &[bool]) -> Option<usize> {
    movable_rows(block, frozen)
        .map(|line| indent_width(&line))
        .min()
}

/// True for a non-blank continuation row at `row` that `frozen` leaves
/// free to move.
fn is_movable(row: usize, line: &str, frozen: &[bool]) -> bool {
    row > 0 && frozen.get(row) != Some(&true) && !line.trim().is_empty()
}

/// Yields each movable non-blank continuation row of `block`, skipping
/// the rows `frozen` marks.
fn movable_rows<'b>(block: &'b str, frozen: &'b [bool]) -> impl Iterator<Item = Line<'b>> {
    block
        .universal_newlines()
        .enumerate()
        .filter(|(row, line)| is_movable(*row, line, frozen))
        .map(|(_, line)| line)
}

/// `block`'s continuation rows moved per `travel`, each blank row and
/// each row `frozen` marks passing through as written.
fn shifted_rows(block: &str, travel: Travel, frozen: &[bool]) -> String {
    let last = block
        .split_inclusive('\n')
        .enumerate()
        .filter(|(row, line)| is_movable(*row, line, frozen))
        .map(|(row, _)| row)
        .last();
    let mut out = String::with_capacity(block.len());
    for (row, line) in block.split_inclusive('\n').enumerate() {
        if !is_movable(row, line, frozen) {
            out.push_str(line);
            continue;
        }
        let placed = travel.placed(indent_width(line), Some(row) == last);
        out.push_str(&" ".repeat(placed));
        out.push_str(&line[leading_indentation(line).len()..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
    #[case("(\n        a,\n        )", &[], Some((0, Some(4))))]
    #[case("(\n  a,\n  )", &[], Some((6, Some(4))))]
    #[case("[\n  a,\n\n  b,\n  ]", &[], Some((6, Some(4))))]
    #[case("(\n      a,\n  )", &[], Some((2, None)))]
    #[case("(\n  a,\n  b)", &[], Some((6, None)))]
    #[case("(\n  a,\n  ).b", &[], Some((6, Some(4))))]
    #[case("(\n    f(\n        a\n    ).b, c)", &[], None)]
    #[case("(\n  a,\n  ).b(\")\")", &[], Some((6, Some(4))))]
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
}

//! Moves a relocated block's continuation rows to the column it lands
//! at. [`block_shift`] reads the move a block makes against its
//! [`Landing`], [`placed_block`] applies it to a source range,
//! [`hung_block_through`] hangs one from the row it lands on instead,
//! [`carried_rows`] lists the rows that move with a value's start,
//! [`seat`] reads what a continuation row sits against, and a row a
//! row-spanning string freezes stays where the source wrote it.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, StringLike,
    helpers::any_over_expr,
    token::{Token, TokenKind},
};
use ruff_python_parser::{Mode, lexer::lex};
use ruff_python_trivia::{PythonWhitespace, leading_indentation};
use ruff_source_file::{Line, LineRanges, OneIndexed, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::{
    primitives::{
        INDENT_STEP,
        edit::apply_inline_edits,
        inline::{display_width, indent_width, spans_rows},
        layout::item_indent,
        tokens::{CLOSERS, is_closer, is_opener, is_row_anchor, open_brackets, tokens_within},
    },
    source::Source,
};

mod blocks;
mod rows;

pub(crate) use blocks::{
    block_shift, carried_rows, frozen_rows, hung_block_through, placed_block, shifted_block,
    spans_a_string_part,
};
pub(crate) use rows::{Seat, is_movable, seat};
use rows::{hanging_travel, movable_floor, movable_rows, rows_beneath, shifted_rows};

/// The opening row of a value, `text` running from the value's start to
/// the end of that row and `column` the display column the value starts
/// at.
#[derive(Clone, Copy)]
struct Head<'a> {
    column: usize,
    text: &'a str,
}

impl<'a> Head<'a> {
    /// The opening row of the value starting at `start`.
    fn of(source: &'a Source, start: TextSize) -> Self {
        Self {
            column: source.column_of(start),
            text: source.slice(source.row_tail(start)),
        }
    }

    /// True where a token that a continuation row aligns to by intent,
    /// per [`is_row_anchor`], starts at display column `at` on this row.
    /// A row ending on an open bracket anchors no row, since the rows
    /// beneath it hang from that bracket.
    fn anchors(self, at: usize) -> bool {
        let Some(offset) = at.checked_sub(self.column) else {
            return false;
        };
        let tokens: Vec<(TokenKind, TextSize)> = row_tokens(self.text).collect();
        !ends_on_an_opener(tokens.iter().copied())
            && tokens.iter().any(|&(kind, start)| {
                is_row_anchor(kind) && display_width(&self.text[..start.to_usize()]) == offset
            })
    }
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
    closer: Option<usize>,
    pub(crate) rows: isize,
}

impl Travel {
    /// The move carrying every continuation row by `rows`.
    pub(crate) fn rigid(rows: isize) -> Self {
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

/// Returns `true` where the last of a row's `tokens` past any trailing
/// comment opens a bracket, the row the rows beneath it hang from. The
/// lexer closes a row that leaves a bracket open with an `Unknown`
/// token, which this passes over as it does trivia.
fn ends_on_an_opener(tokens: impl Iterator<Item = (TokenKind, TextSize)>) -> bool {
    tokens
        .map(|(kind, _)| kind)
        .filter(|kind| !kind.is_trivia() && *kind != TokenKind::Unknown)
        .last()
        .is_some_and(is_opener)
}

/// Lexes `row`, yielding the kind and start of each token ahead of the
/// row's end.
fn row_tokens(row: &str) -> impl Iterator<Item = (TokenKind, TextSize)> + '_ {
    let mut lexer = lex(row, Mode::Expression);
    std::iter::from_fn(move || {
        let kind = lexer.next_token();
        (!matches!(kind, TokenKind::EndOfFile | TokenKind::Newline))
            .then(|| (kind, lexer.current_range().start()))
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::TextLen;

    use super::*;
    use crate::testing::parse;

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
    #[case::aligned_beneath_its_start("x = f(a,\n      b)\n", &["      b)"])]
    #[case::hung_one_step_in("x = f(\n    a,\n)\n", &[])]
    #[case::hung_above_a_shallower_closer("x = [\n    a,\n   ]\n", &[])]
    #[case::one_step_in_beneath_a_token("x = f(a,\n    b)\n", &["    b)"])]
    #[case::one_step_in_beneath_a_bracket("x = [a,\n    b]\n", &[])]
    #[case::beneath_a_token_above_a_shallower_closer(
        "x = f(a,\n      g(b,\n        c),\n  d,\n)\n",
        &["      g(b,", "        c),"]
    )]
    #[case::past_a_row_spanning_string(
        "x = f(\"\"\"a\n  b\"\"\",\n      c)\n",
        &["      c)"]
    )]
    #[case::indented_with_a_tab("x = f(a,\n      b,\n\t\t\t\t\t\tc)\n", &["      b,"])]
    #[case::on_one_row("x = f(a, b)\n", &[])]
    fn carried_rows_lists_each_row_moving_with_the_block_start(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let source = parse(src);
        let block = TextRange::new(TextSize::new(4), src.text_len() - TextSize::of('\n'));
        let rows: Vec<&str> = carried_rows(&source, block, 0)
            .into_iter()
            .map(|row| source.slice(row))
            .collect();
        assert_eq!(rows, expected);
    }

    #[rstest]
    #[case::a_name_past_the_start("a + b", 4, 8, true)]
    #[case::the_value_itself("\"a\"", 8, 8, true)]
    #[case::a_comma("a, b", 0, 1, false)]
    #[case::a_comment("a  # c", 0, 3, false)]
    #[case::past_every_token("a", 0, 5, false)]
    #[case::left_of_the_value("a", 4, 0, false)]
    #[case::a_row_left_open("f(a, g(", 0, 2, false)]
    #[case::a_row_left_open_before_a_comment("foo(  # c", 4, 4, false)]
    fn head_anchors_a_row_at_the_column_one_of_its_tokens_starts_at(
        #[case] text: &str,
        #[case] column: usize,
        #[case] at: usize,
        #[case] expected: bool,
    ) {
        assert_eq!(Head { column, text }.anchors(at), expected);
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

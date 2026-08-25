//! Moves a relocated block's continuation rows to the column it lands
//! at. [`block_shift`] reads the move a block makes against its
//! [`Landing`], [`placed_block`] applies it to a source range,
//! [`hung_block_through`] hangs one from the row it lands on instead,
//! and a row a row-spanning string freezes stays where the source
//! wrote it.

mod blocks;
mod rows;

use rows::{hanging_travel, movable_floor, shifted_rows};

pub(crate) use blocks::{
    block_shift, frozen_rows, hung_block_through, placed_block, shifted_block, spans_a_string_part,
};

use ruff_diagnostics::Edit;
use ruff_python_ast::{Expr, StringLike, helpers::any_over_expr, token::TokenKind};
use ruff_python_parser::{Mode, lexer::lex};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::{Line, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        edit::apply_inline_edits,
        inline::indent_width,
        layout::item_indent,
        tokens::{CLOSERS, OPENERS, is_closer, is_opener},
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

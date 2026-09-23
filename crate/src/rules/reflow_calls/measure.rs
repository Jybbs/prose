//! The columns an explode decision reads: where a call's `(` lands once
//! the walk's earlier edits place the text ahead of it, the indent an
//! exploded closing bracket drops to, whether a literal holding a call
//! is one `reflow-collections` expands once its row lands, and the
//! layout a construct takes where a relocated walk lands it.

use std::borrow::Cow;

use ruff_python_ast::{Expr, ExprCall, token::TokenKind};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use super::{CollectionLayout, Exploder};
use crate::primitives::{
    edit::{apply_inline_edits, placed_head},
    inline::{end_column, indent_width, last_line, settled_width, spans_rows},
    tokens::{is_closer, is_opener},
    travel::{Travel, shifted_block},
};

impl<'a> Exploder<'a> {
    /// The column `offset` reaches once this walk's earlier edits place
    /// the text ahead of it and the padding rule settles its row, a row
    /// past the region's opening one moved by `line_shift`. In a module
    /// walk, a `reserved` offset starts from the column `align_equals`
    /// shifts its row to.
    fn placed_column(&self, offset: TextSize, reserved: bool) -> usize {
        let placed = placed_head(self.source, &self.edits, offset, self.region.start());
        let row_start = self.source.text().line_start(offset).max(
            offset
                .checked_sub(last_line(&placed).text_len())
                .unwrap_or_default(),
        );
        let row = TextRange::new(row_start, offset);
        let mut column = settled_width(
            self.source,
            self.padding,
            row,
            end_column(&placed, self.origin_column),
        );
        if spans_rows(&placed) {
            column = column.saturating_add_signed(self.line_shift);
        }
        if self.indent.is_some() || !reserved {
            return column;
        }
        self.reservations.column(offset, || column)
    }

    /// An offset on the row whose indent the row carrying `offset`
    /// settles to: the row carrying the outermost opener among the
    /// brackets open at the row's start that close ahead of `offset`,
    /// read again from that row, or `offset` itself where none does.
    /// One pass back along the logical line carries the closers a row
    /// leaves unmatched into the rows above it and stops at the first
    /// earlier row reached with none pending.
    fn settled_row_anchor(&self, offset: TextSize) -> TextSize {
        let text = self.source.text();
        let mut anchor = offset;
        let mut row_start = text.line_start(offset);
        let mut pending = 0_usize;
        let before = self.source.tokens().before(offset);
        for token in before
            .iter()
            .rev()
            .take_while(|token| token.kind() != TokenKind::Newline)
        {
            if pending == 0 && token.start() < row_start {
                break;
            }
            if is_closer(token.kind()) {
                pending += 1;
            } else if is_opener(token.kind()) && pending > 0 {
                pending -= 1;
                if pending == 0 && token.start() < row_start {
                    anchor = token.start();
                    row_start = text.line_start(anchor);
                }
            }
        }
        anchor
    }

    /// True where `reflow-collections` expands `literal` once its row
    /// lands, per [`Settings::expands`](crate::primitives::one_row::Settings::expands),
    /// leaving every call inside to the reshape that rule runs where the
    /// entries land.
    pub(super) fn expands_later(&self, literal: &Expr) -> bool {
        self.expands_literals
            && self
                .source
                .expandable_literals()
                .binary_search_by_key(&literal.start(), Ranged::start)
                .is_ok()
            && self.one_row.expands(
                self.source,
                literal,
                literal.into(),
                self.placed_column(literal.start(), true),
                self.row_tail(literal.end()),
                self.padding,
            )
    }

    /// The indent an exploded closing bracket drops to for the construct
    /// opening at `offset`: this walk's own indent where the construct
    /// settles on the row the region opens on, and otherwise the placed
    /// indent of the row [`Self::settled_row_anchor`] resolves for its
    /// opening bracket.
    pub(super) fn indent_for(&self, offset: TextSize) -> usize {
        let anchor = self.settled_row_anchor(offset).max(self.region.start());
        if let Some(indent) = self.indent
            && self.source.same_line(self.region.start(), anchor)
        {
            return indent;
        }
        let placed = placed_head(self.source, &self.edits, anchor, self.region.start());
        indent_width(last_line(&placed))
    }

    /// `expr`'s replacement under `layout` once its row lands, measured
    /// at the indent its row takes after this walk's rows move and
    /// rendered at the indent before that move, so the move carries
    /// each of its rows into place.
    pub(super) fn laid_out(&self, layout: &dyn CollectionLayout, expr: &Expr) -> Option<String> {
        let column = self.placed_column(expr.start(), true);
        let tail = self.row_tail(expr.end());
        let indent = self
            .indent_for(expr.start())
            .saturating_add_signed(self.line_shift);
        let text = layout.laid_out(expr, column, indent, tail)?;
        Some(
            match shifted_block(&text, Travel::rigid(-self.line_shift)) {
                Cow::Borrowed(_) => text,
                Cow::Owned(moved) => moved,
            },
        )
    }

    /// The column `call`'s `(` reaches once this walk's earlier edits
    /// place the text ahead of it, a call whose rendered callee holds
    /// no break starting from the column `align_equals` shifts its row
    /// to in a module walk.
    pub(super) fn open_paren_column(&self, call: &ExprCall) -> usize {
        let callee = apply_inline_edits(self.source, call.func.range(), &self.edits);
        self.placed_column(call.arguments.start(), !spans_rows(&callee))
    }
}

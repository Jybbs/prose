//! The columns an explode decision reads: where a call's `(` lands once
//! the walk's earlier edits place the text ahead of it, and the indent
//! an exploded closing `)` drops to.

use std::borrow::Cow;

use ruff_python_ast::{ExprCall, token::TokenKind};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::Exploder;
use crate::primitives::{
    edit::apply_inline_edits,
    inline::{end_column, indent_width},
    tokens::{is_closer, is_opener},
};

impl<'a> Exploder<'a> {
    /// The indent an exploded closing `)` drops to for `call`: this
    /// walk's own indent where the argument list settles on the row the
    /// region opens on, and otherwise the placed indent of the row
    /// [`Self::settled_row_anchor`] resolves for the `(`.
    pub(super) fn indent_for(&self, call: &ExprCall) -> usize {
        let anchor = self
            .settled_row_anchor(call.arguments.start())
            .max(self.region.start());
        if let Some(indent) = self.indent
            && self.source.same_line(self.region.start(), anchor)
        {
            return indent;
        }
        let placed = self.placed_head(anchor);
        indent_width(placed.rsplit('\n').next().unwrap_or(&placed))
    }

    /// The column `call`'s `(` reaches once this walk's earlier edits
    /// place the text ahead of it, a row past the region's opening one
    /// moved by `line_shift`. In a module walk, a call whose rendered
    /// callee holds no break starts from the column `align_equals`
    /// shifts its row to.
    pub(super) fn open_paren_column(&self, call: &ExprCall) -> usize {
        let offset = call.arguments.start();
        let placed = self.placed_head(offset);
        let mut column = end_column(&placed, self.origin_column);
        if placed.contains('\n') {
            column = column.saturating_add_signed(self.line_shift);
        }
        let callee = apply_inline_edits(self.source, call.func.range(), &self.edits);
        if self.indent.is_some() || callee.contains('\n') {
            return column;
        }
        self.reservations.column(offset, || column)
    }

    /// The text ahead of `offset` on its logical line, clipped to this
    /// walk's region and rendered with the edits the walk emitted so
    /// far.
    fn placed_head(&self, offset: TextSize) -> Cow<'a, str> {
        let start = self
            .source
            .logical_line_start(offset)
            .start()
            .max(self.region.start());
        apply_inline_edits(self.source, TextRange::new(start, offset), &self.edits)
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
}

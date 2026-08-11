//! The column measures an explode decision reads: where a call's `(`
//! lands once its callee renders, and the indent an exploded closing
//! `)` drops to.

use std::borrow::Cow;

use ruff_python_ast::ExprCall;
use ruff_text_size::{Ranged, TextSize};

use super::Exploder;
use crate::primitives::{
    edit::apply_inline_edits,
    inline::{end_column, indent_width},
};

impl<'a> Exploder<'a> {
    /// The column `offset` reaches once this walk's subtree is placed,
    /// which is the source column plus `line_shift` on every line past
    /// the opening one.
    fn column_of(&self, offset: TextSize) -> usize {
        if self.source.same_line(self.origin, offset) {
            self.origin_column + self.source.width_between(self.origin, offset)
        } else {
            self.source
                .column_of(offset)
                .saturating_add_signed(self.line_shift)
        }
    }

    /// `call`'s callee rendered with the edits this walk emitted so
    /// far, the text the argument list measures against.
    pub(super) fn callee_text(&self, call: &'a ExprCall) -> Cow<'a, str> {
        apply_inline_edits(self.source, call.func.range(), &self.edits)
    }

    /// The indent an exploded closing `)` drops to for `call`, this
    /// walk's own indent for a call opening on the row a relocated value
    /// starts on, and otherwise the indent of the row carrying the
    /// argument list's `(` as this walk has already placed it, which an
    /// earlier edit on the same statement may have moved. A callee
    /// spanning rows leaves that `(` on a row of its own, deeper than
    /// the one the call opens on. A call on a later row of a relocated
    /// value reads that placed row, which the caller's own move then
    /// carries with the rest of the block.
    pub(super) fn indent_for(&self, call: &ExprCall) -> usize {
        if let Some(indent) = self.indent
            && self.source.same_line(self.origin, call.start())
        {
            return indent;
        }
        let head = self.source.logical_line_start(call.arguments.start());
        let placed = apply_inline_edits(self.source, head, &self.edits);
        indent_width(placed.rsplit('\n').next().unwrap_or(&placed))
    }

    /// The column `call`'s `(` reaches once `callee` renders. A call
    /// that is itself an aligned value whose rendered callee holds no
    /// break starts from the column `align_equals` shifts it to, and
    /// every other call from this walk's own placement. A rendered
    /// callee spanning lines ends on a row `line_shift` moves.
    pub(super) fn open_paren_column(&self, call: &ExprCall, callee: &str) -> usize {
        let gap = self
            .source
            .width_between(call.func.end(), call.arguments.start());
        if callee.contains('\n') {
            return end_column(callee, 0).saturating_add_signed(self.line_shift) + gap;
        }
        let start = call.start();
        let head = self.reservations.column(start, || self.column_of(start));
        end_column(callee, head) + gap
    }
}

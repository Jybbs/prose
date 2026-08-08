//! The column measures an explode decision reads: where a call's `(`
//! lands once its callee renders, and the indent an exploded closing
//! `)` drops to.

use std::borrow::Cow;

use ruff_python_ast::ExprCall;
use ruff_text_size::{Ranged, TextSize};

use super::Exploder;
use crate::primitives::{edit::apply_inline_edits, inline::end_column};

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
    /// walk's own indent inside a relocated value and the call's source
    /// line indent otherwise.
    pub(super) fn indent_for(&self, call: &ExprCall) -> usize {
        self.indent
            .unwrap_or_else(|| self.source.line_indent_width(call.start()))
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

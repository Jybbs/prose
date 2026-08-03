//! The column and width measures an explode decision reads: where a
//! call's `(` lands once its callee renders, and whether the argument
//! list joined onto that row crosses the budget.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_python_ast::{ArgOrKeyword, Arguments, ExprCall};
use ruff_text_size::{Ranged, TextSize};

use super::Exploder;
use crate::primitives::{
    edit::apply_inline_edits,
    inline::{end_column, opening_width},
    reserve::settled_column,
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

    /// `arguments` rendered on one line, joined by `", "` inside the
    /// parens. A named keyword measures at its canonical `name=value`
    /// rather than at whatever padding `align_equals` gave it.
    fn inline_args(&self, arguments: &Arguments) -> String {
        format!(
            "({})",
            arguments
                .iter_source_order()
                .map(|arg| match arg {
                    ArgOrKeyword::Arg(expr) => Cow::Borrowed(self.source.slice(expr)),
                    ArgOrKeyword::Keyword(kw) => match &kw.arg {
                        Some(name) => {
                            Cow::Owned(format!("{name}={}", self.source.slice(&kw.value)))
                        }
                        None => Cow::Borrowed(self.source.slice(kw)),
                    },
                })
                .join(", "),
        )
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
        let head = settled_column(self.reservations, start, || self.column_of(start));
        end_column(callee, head) + gap
    }

    /// True when `arguments` rendered inline from `column` crosses
    /// `code_line_length`. An argument that itself spans lines caps the
    /// measure at the row the join opens.
    pub(super) fn overflows_line(&self, arguments: &Arguments, column: usize) -> bool {
        column + opening_width(&self.inline_args(arguments)) > self.code_line_length
    }
}

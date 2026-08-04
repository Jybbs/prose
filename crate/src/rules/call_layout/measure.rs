//! The column and width measures an explode decision reads: where a
//! call's `(` lands once its callee renders, and whether the argument
//! list joined onto that row crosses the budget. A fractured argument
//! list beneath the one being measured joins first, so the row reads
//! the width the rejoin path settles on rather than the source's.

use std::{borrow::Cow, cmp::Reverse};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    ArgOrKeyword, Arguments, Expr, ExprCall,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};

use super::Exploder;
use crate::{
    primitives::{
        edit::apply_inline_edits,
        inline::{end_column, opening_width},
        layout::is_column_shaped,
        reserve::settled_column,
    },
    source::Source,
};

/// Joins every fractured argument list beneath the visited expression
/// onto one line, one replacement edit per list. A column-shaped list,
/// one carrying a comment, and one the count trigger explodes all hold
/// their break.
struct FractureJoiner<'a> {
    cap: Option<usize>,
    edits: Vec<Edit>,
    source: &'a Source,
}

impl<'ast> AstVisitor<'ast> for FractureJoiner<'_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        walk_expr(self, expr);
        let Expr::Call(call) = expr else {
            return;
        };
        let range = call.arguments.range();
        if call.arguments.is_empty()
            || self.cap.is_some_and(|cap| call.arguments.len() > cap)
            || !self.source.contains_line_break(range)
            || is_column_shaped(self.source.slice(range))
            || self.source.intersects_comment(call.arguments.inner_range())
        {
            return;
        }
        self.edits.push(Edit::range_replacement(
            join_args(self.source, self.cap, &call.arguments),
            range,
        ));
    }
}

/// `expr`'s text with every fractured argument list beneath it joined
/// onto one line, the shape the rejoin path settles on. A
/// column-shaped list keeps its break, so an enclosing measure still
/// reads it as spanning lines.
///
/// The walk reaches a nested list twice over, once on its own and once
/// inside the join its parent renders, so the edits sort outermost
/// first and every range a kept edit already covers drops. That leaves
/// the ascending, disjoint run `apply_inline_edits` splices.
fn settled<'a>(source: &'a Source, cap: Option<usize>, expr: &Expr) -> Cow<'a, str> {
    let range = expr.range();
    if !source.contains_line_break(range) {
        return Cow::Borrowed(source.slice(range));
    }
    let mut joiner = FractureJoiner {
        cap,
        edits: Vec::new(),
        source,
    };
    joiner.visit_expr(expr);
    joiner
        .edits
        .sort_by_key(|edit| (edit.start(), Reverse(edit.end())));
    let mut outermost: Vec<Edit> = Vec::new();
    for edit in joiner.edits {
        if outermost
            .last()
            .is_none_or(|kept| kept.end() <= edit.start())
        {
            outermost.push(edit);
        }
    }
    apply_inline_edits(source, range, &outermost)
}

/// `arguments` joined by `", "` inside the parens, each argument
/// settled so a nested fracture reads at its joined width.
fn join_args(source: &Source, cap: Option<usize>, arguments: &Arguments) -> String {
    format!(
        "({})",
        arguments
            .iter_source_order()
            .map(|arg| match arg {
                ArgOrKeyword::Arg(expr) => settled(source, cap, expr),
                ArgOrKeyword::Keyword(kw) => match &kw.arg {
                    Some(name) => Cow::Owned(format!("{name}={}", settled(source, cap, &kw.value))),
                    None => Cow::Borrowed(source.slice(kw)),
                },
            })
            .join(", "),
    )
}

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
    /// rather than at whatever padding `align_equals` gave it, and a
    /// value carrying a fractured call measures at that call joined.
    pub(super) fn inline_args(&self, arguments: &Arguments) -> String {
        join_args(self.source, self.cap, arguments)
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

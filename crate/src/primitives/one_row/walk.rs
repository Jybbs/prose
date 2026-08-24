//! The walk closing every fractured bracketed construct beneath one
//! leaf expression, collecting the join each one takes.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};

use super::{Column, render::Writer};
use crate::primitives::layout::{is_collapsible, is_fractured};

/// Collects the join closing each fractured bracketed construct beneath
/// one leaf expression, a call's argument list taking its one-row
/// `(...)` form and a literal, subscript, or comprehension its own.
/// `reachable` clears where any of them reaches no single row, leaving
/// the whole leaf without a form.
pub(super) struct Joiner<'a, 'w> {
    pub(super) edits: Vec<Edit>,
    pub(super) reachable: bool,
    pub(super) writer: &'w Writer<'a>,
}

impl Joiner<'_, '_> {
    /// The one-row text closing `range`, `None` where none exists. An
    /// argument list already carrying the flush column shape the explode
    /// path emits holds that shape, the same reading `is_fractured`
    /// gives a list a join could close.
    fn closed(&self, expr: &Expr, range: TextRange) -> Option<String> {
        let source = self.writer.source;
        let Expr::Call(call) = expr else {
            return self
                .writer
                .formed(expr, range, Column::Holds)
                .map(Cow::into_owned);
        };
        is_fractured(source, range)
            .then(|| self.writer.settings.arguments_form(source, &call.arguments))
            .flatten()
    }

    /// The range a fractured bracketed construct at `expr` closes over,
    /// `None` where `expr` carries no such construct or holds no break.
    fn fractured(&self, expr: &Expr) -> Option<TextRange> {
        let range = match expr {
            Expr::Call(call) => call.arguments.range(),
            _ if is_collapsible(expr) => expr.range(),
            _ => return None,
        };
        self.writer
            .source
            .contains_line_break(range)
            .then_some(range)
    }
}

impl<'ast> AstVisitor<'ast> for Joiner<'_, '_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if !self.reachable {
            return;
        }
        walk_expr(self, expr);
        let Some(range) = self.fractured(expr) else {
            return;
        };
        match self.closed(expr, range) {
            Some(form) => self.edits.push(Edit::range_replacement(form, range)),
            None => self.reachable = false,
        }
    }
}

//! Walks class bodies tracking the frame each `super` call sits in.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, Stmt, StmtClassDef,
    visitor::{Visitor, walk_expr},
};
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::*;

/// One enclosing callable. `receiver` names the parameter the bare form
/// reads as the instance, `None` for a comprehension and for a callable
/// whose leading slot is keyword-only or variadic. `scope` carries the
/// `def` whose locals the class name resolves against, `None` for a
/// lambda and a comprehension. `class_depth` is the enclosing class
/// count where the callable opened.
pub(super) struct Frame<'a> {
    pub(super) class_depth: usize,
    pub(super) receiver: Option<&'a str>,
    pub(super) scope: Option<&'a Stmt>,
}

/// Collects one fix group per rewritable `super(...)` call, carrying the
/// enclosing class stack, the callable frame stack the walk maintains,
/// and the start of the statement under visit.
pub(super) struct Walker<'a> {
    pub(super) analysis: &'a BindingAnalysis,
    pub(super) classes: Vec<&'a StmtClassDef>,
    pub(super) frames: Vec<Frame<'a>>,
    pub(super) groups: Vec<Vec<Edit>>,
    pub(super) source: &'a Source,
    pub(super) statement: TextSize,
}

impl<'a> Walker<'a> {
    /// Pushes a callable frame, runs `walk` inside it, and pops it.
    fn in_frame(
        &mut self,
        receiver: Option<&'a str>,
        scope: Option<&'a Stmt>,
        walk: impl FnOnce(&mut Self),
    ) {
        self.frames.push(Frame {
            class_depth: self.classes.len(),
            receiver,
            scope,
        });
        walk(self);
        self.frames.pop();
    }

    /// The edits deleting `call`'s arguments and re-seating any later
    /// row of the logical line aligned to text the deletion moves,
    /// `None` where the bare form would resolve a different class or
    /// instance, or none at all, and `None` where a stranded row sits
    /// inside a string no move re-seats. A span written across rows
    /// joins the rows it spans, so the rows below it hold.
    fn rewrite(&self, call: &ExprCall) -> Option<Vec<Edit>> {
        if call.func.as_name_expr()?.id.as_str() != "super" || !call.arguments.keywords.is_empty() {
            return None;
        }
        let [Expr::Name(class_arg), Expr::Name(instance_arg)] = &*call.arguments.args else {
            return None;
        };
        let frame = self.frames.last()?;
        let class = self.classes.last()?;
        let depth = self.classes.len();
        let name = class_arg.id.as_str();
        let span = call.arguments.inner_range();
        if frame.class_depth != depth
            || frame.receiver != Some(instance_arg.id.as_str())
            || !names_the_class(name, class, depth)
            || self.shadows(name)
            || is_slots_dataclass(class)
            || self.source.intersects_comment(span)
        {
            return None;
        }
        let tail = self.source.logical_line_tail(span.end());
        let column = self.source.column_of(span.start());
        if self.strands_a_string_row(tail, column) {
            return None;
        }
        let removal = Edit::range_deletion(span);
        let mut edits = Vec::new();
        if !self.source.contains_line_break(span) {
            push_reseat_edits(self.source, std::slice::from_ref(&removal), &mut edits);
        }
        insert_edit(&mut edits, removal);
        Some(edits)
    }

    /// True when an enclosing `def` binds `name` in its own scope,
    /// leaving the argument reading that binding rather than the class.
    fn shadows(&self, name: &str) -> bool {
        self.frames
            .iter()
            .filter_map(|frame| frame.scope)
            .any(|scope| self.analysis.scope_binds(scope, name))
    }

    /// True when a row of `tail` inside a row-spanning string opens at
    /// `column` or past it, a row no move re-seats.
    fn strands_a_string_row(&self, tail: TextRange, column: usize) -> bool {
        self.source
            .slice(tail)
            .universal_newlines()
            .zip(frozen_rows(self.source, tail))
            .any(|(line, frozen)| frozen && indent_width(&line) >= column)
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(call) => {
                if let Some(edits) = self.rewrite(call) {
                    self.groups.push(edits);
                }
                walk_expr(self, expr);
            }
            Expr::DictComp(_) | Expr::Generator(_) | Expr::ListComp(_) | Expr::SetComp(_) => {
                self.in_frame(None, None, |walker| walk_expr(walker, expr));
            }
            Expr::Lambda(lambda) => {
                let first = lambda
                    .parameters
                    .as_deref()
                    .and_then(first_positional)
                    .map(|p| p.name().as_str());
                self.in_frame(first, None, |walker| walk_expr(walker, expr));
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.statement = stmt.start();
        match stmt {
            Stmt::ClassDef(class) => {
                self.classes.push(class);
                walk_stmt(self, stmt);
                self.classes.pop();
            }
            Stmt::FunctionDef(function) => {
                let first = first_positional(&function.parameters).map(|p| p.name().as_str());
                self.in_frame(first, Some(stmt), |walker| walk_stmt(walker, stmt));
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

/// True when `name` reaches `class` from a method body: the implicit
/// `__class__` cell at any nesting, or the class's own name where
/// `depth` counts one enclosing class, since a nested class binds its
/// name outside the method's reach.
pub(super) fn names_the_class(name: &str, class: &StmtClassDef, depth: usize) -> bool {
    name == "__class__" || (depth == 1 && name == class.name.as_str())
}

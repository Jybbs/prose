//! The body scope a statement sits in (module, class, or function) and
//! the sub-bodies a compound statement opens.

use ruff_python_ast::{ExceptHandler, Stmt};
use ruff_text_size::TextRange;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum BodyScope {
    Class,
    Function,
    Module,
}

/// Returns one `(body, outer)` pair per sub-body of a compound statement.
/// `outer` carries the enclosing arm's range, which bounds a leading-comment
/// scan for the body's first item. Empty sub-bodies are returned as-is and
/// skipped by the caller.
pub(crate) fn compound_sub_bodies(stmt: &Stmt) -> Vec<(&[Stmt], TextRange)> {
    match stmt {
        Stmt::For(s) => vec![(s.body.as_slice(), s.range), (s.orelse.as_slice(), s.range)],
        Stmt::If(s) => std::iter::once((s.body.as_slice(), s.range))
            .chain(
                s.elif_else_clauses
                    .iter()
                    .map(|c| (c.body.as_slice(), c.range)),
            )
            .collect(),
        Stmt::Match(s) => s
            .cases
            .iter()
            .map(|c| (c.body.as_slice(), c.range))
            .collect(),
        Stmt::Try(s) => std::iter::once((s.body.as_slice(), s.range))
            .chain(
                s.handlers
                    .iter()
                    .map(|ExceptHandler::ExceptHandler(h)| (h.body.as_slice(), h.range)),
            )
            .chain([
                (s.orelse.as_slice(), s.range),
                (s.finalbody.as_slice(), s.range),
            ])
            .collect(),
        Stmt::While(s) => vec![(s.body.as_slice(), s.range), (s.orelse.as_slice(), s.range)],
        Stmt::With(s) => vec![(s.body.as_slice(), s.range)],
        _ => Vec::new(),
    }
}

/// Returns the body and scope a class or function definition opens.
/// `None` for every other statement.
pub(crate) fn scoped_body(stmt: &Stmt) -> Option<(&[Stmt], BodyScope)> {
    match stmt {
        Stmt::ClassDef(c) => Some((&c.body, BodyScope::Class)),
        Stmt::FunctionDef(f) => Some((&f.body, BodyScope::Function)),
        _ => None,
    }
}

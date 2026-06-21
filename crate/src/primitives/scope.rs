//! The body scope a statement sits in: module, class, or function.

use ruff_python_ast::Stmt;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum BodyScope {
    Class,
    Function,
    Module,
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

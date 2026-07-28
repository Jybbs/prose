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

/// Returns one `(body, outer)` pair per non-empty sub-body of a compound
/// statement. `outer` carries the enclosing arm's range, which bounds a
/// leading-comment scan for the body's first item.
pub(crate) fn compound_sub_bodies(stmt: &Stmt) -> Vec<(&[Stmt], TextRange)> {
    let mut bodies = match stmt {
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
    };
    bodies.retain(|(body, _)| !body.is_empty());
    bodies
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case("for i in items:\n    a = 1\n", 1)]
    #[case("try:\n    a = 1\nexcept ValueError:\n    b = 2\n", 2)]
    #[case("while flag:\n    a = 1\n", 1)]
    fn compound_sub_bodies_drops_an_absent_arm(#[case] src: &str, #[case] arms: usize) {
        let source = parse(src);
        assert_eq!(compound_sub_bodies(&source.ast().body[0]).len(), arms);
    }
}

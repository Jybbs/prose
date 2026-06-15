//! Strips a function's `-> None` return annotation. Leaves a `None`
//! nested in a larger annotation (`int | None`, `Callable[..., None]`),
//! every parameter annotation, and a declaration-only `...` stub's own
//! `-> None` in place.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt, StmtFunctionDef,
    helpers::body_without_leading_docstring,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{edit::singleton_groups, range::return_annotation_range},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct StripNoneReturn;

impl StripNoneReturn {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for StripNoneReturn {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut walker = Walker {
            edits: Vec::new(),
            source,
        };
        walker.visit_body(&source.ast().body);
        singleton_groups(walker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl Walker<'_> {
    /// Deletes the ` -> None` span from `(`'s close through the
    /// annotation, parens included.
    fn strip(&mut self, fd: &StmtFunctionDef) {
        if let Some(returns) = fd.returns.as_deref()
            && returns.is_none_literal_expr()
            && !is_ellipsis_stub(&fd.body)
        {
            let annotation = return_annotation_range(returns, fd, self.source.tokens());
            let span = TextRange::new(fd.parameters.range().end(), annotation.end());
            self.edits.push(Edit::range_deletion(span));
        }
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.strip(fd);
        }
        walk_stmt(self, stmt);
    }
}

/// A declaration-only stub, a lone `...` statement past an optional
/// leading docstring.
fn is_ellipsis_stub(body: &[Stmt]) -> bool {
    matches!(
        body_without_leading_docstring(body),
        [Stmt::Expr(stmt)] if stmt.value.is_ellipsis_literal_expr()
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    fn strip_groups(src: &str) -> Vec<Vec<Edit>> {
        StripNoneReturn.apply(&parse(src))
    }

    #[test]
    fn deletes_the_arrow_through_the_none_literal() {
        let source = parse("def f() -> None:\n    pass\n");
        let groups = StripNoneReturn.apply(&source);
        let edit = &groups[0][0];
        assert!(edit.is_deletion());
        assert_eq!(&source.text()[edit.range()], " -> None");
    }

    #[rstest]
    fn keeps_none_on_a_declaration_stub(
        #[values(
            "def f() -> None: ...\n",
            "@overload\ndef f(x: int) -> None: ...\n",
            "class P:\n    def m(self) -> None: ...\n",
            "def f() -> None:\n    \"\"\"doc\"\"\"\n    ...\n"
        )]
        src: &str,
    ) {
        assert!(strip_groups(src).is_empty());
    }

    #[rstest]
    fn leaves_a_non_bare_none_return_in_place(
        #[values(
            "def f() -> int | None:\n    return 1\n",
            "def f() -> None | int:\n    return 1\n",
            "def f() -> int:\n    return 1\n",
            "def f():\n    pass\n"
        )]
        src: &str,
    ) {
        assert!(strip_groups(src).is_empty());
    }

    #[rstest]
    fn strips_when_the_body_is_not_a_lone_ellipsis(
        #[values(
            "def f() -> None:\n    x = 1\n    ...\n",
            "def f() -> None:\n    print()\n",
            "def f() -> None:\n    \"\"\"doc\"\"\"\n"
        )]
        src: &str,
    ) {
        assert!(!strip_groups(src).is_empty());
    }
}

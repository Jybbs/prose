//! Strips a function's `-> None` return annotation. Leaves a `None`
//! nested in a larger annotation (`int | None`, `Callable[..., None]`),
//! every parameter annotation, and a declaration-only `...` stub's own
//! `-> None` in place.

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, StmtFunctionDef, helpers::body_without_leading_docstring};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        edit::singleton_groups, range::return_annotation_range, walk::filter_map_over_stmts,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct StripNoneReturn;

impl StripNoneReturn {
    pub(crate) const MESSAGE: &'static str = "drop a redundant `-> None` return annotation";

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for StripNoneReturn {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(filter_map_over_stmts(&source.ast().body, |stmt| {
            strip(source, stmt.as_function_def_stmt()?)
        }))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
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

/// The deletion taking the ` -> None` span from `(`'s close through the
/// annotation, parens included, `None` where the annotation stays.
fn strip(source: &Source, fd: &StmtFunctionDef) -> Option<Edit> {
    let returns = fd.returns.as_deref()?;
    if !returns.is_none_literal_expr() || is_ellipsis_stub(&fd.body) {
        return None;
    }
    let annotation = return_annotation_range(returns, fd, source.tokens());
    Some(Edit::range_deletion(TextRange::new(
        fd.parameters.range().end(),
        annotation.end(),
    )))
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

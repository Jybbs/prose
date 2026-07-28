//! Pure readers over the decorators a definition carries.

use ruff_python_ast::{
    Arguments, Decorator, Stmt, StmtClassDef, StmtFunctionDef, helpers::map_callable,
};

use crate::primitives::binding::tail_identifier;

/// The argument list a decorator carries when applied as a call, `None`
/// where it is applied bare.
pub(super) fn decorator_arguments(decorator: &Decorator) -> Option<&Arguments> {
    decorator
        .expression
        .as_call_expr()
        .map(|call| &call.arguments)
}

/// The trailing identifier of a decorator's callable, so `@a.b.c(...)`
/// and `@c` both read as `c`. `None` where the callable is neither a
/// name nor an attribute access.
pub(crate) fn decorator_simple_name(decorator: &Decorator) -> Option<&str> {
    tail_identifier(map_callable(&decorator.expression))
}

/// True when `stmt` is a class or function definition carrying at least
/// one decorator.
pub(crate) fn is_decorated(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::ClassDef(StmtClassDef { decorator_list, .. })
        | Stmt::FunctionDef(StmtFunctionDef { decorator_list, .. }) => !decorator_list.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, parse};

    #[rstest]
    #[case("@wraps\ndef f(): pass\n", None)]
    #[case("@wraps()\ndef f(): pass\n", Some(0))]
    #[case("@wraps(other)\ndef f(): pass\n", Some(1))]
    #[case("@pytest.mark.parametrize(\"a\", [1])\ndef f(): pass\n", Some(2))]
    fn decorator_arguments_reads_a_called_decorators_argument_list(
        #[case] src: &str,
        #[case] expected: Option<usize>,
    ) {
        let source = parse(src);
        let decorator = first_def(&source)
            .decorator_list
            .first()
            .expect("one decorator");
        assert_eq!(
            decorator_arguments(decorator).map(|a| a.args.len()),
            expected
        );
    }

    #[rstest]
    #[case("@property\ndef f(): pass\n", Some("property"))]
    #[case("@functools.cached_property\ndef f(): pass\n", Some("cached_property"))]
    #[case("@click.option(\"--name\")\ndef f(): pass\n", Some("option"))]
    #[case(
        "@pytest.mark.parametrize(\"a\", [1])\ndef f(): pass\n",
        Some("parametrize")
    )]
    #[case("@functools.wraps(other)\ndef f(): pass\n", Some("wraps"))]
    #[case("@(some_factory())()\ndef f(): pass\n", None)]
    fn decorator_simple_name_reads_the_rightmost_segment(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let decorator = first_def(&source)
            .decorator_list
            .first()
            .expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), expected);
    }

    #[rstest]
    #[case("@dec\nclass C: pass\n", true)]
    #[case("class C: pass\n", false)]
    #[case("@dec\ndef f(): pass\n", true)]
    #[case("def f(): pass\n", false)]
    #[case("X = 1\n", false)]
    fn is_decorated_reads_a_definitions_decorator_list(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        assert_eq!(is_decorated(&source.ast().body[0]), expected);
    }
}

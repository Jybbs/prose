//! The regions of a function body whose interior repeats, runs under a
//! guard, or opens a scope of its own.

use ruff_python_ast::{
    ExceptHandler, Expr, ExprDictComp, ExprGenerator, ExprListComp, ExprSetComp, Stmt,
};
use ruff_text_size::{Ranged, TextRange};

use crate::primitives::{
    range::blocks_span,
    scope::sub_bodies,
    walk::{Descent, filter_map_over_exprs, filter_map_over_stmts},
};

/// Every span in `body` that repeats its interior, guards it, or opens
/// a scope for it, covering every arm a loop, a `try`, a `with`, or a
/// nested `def` opens, a `while` test, an `except` clause's type
/// expression, a comprehension outside its first iterable, and a
/// lambda's body.
pub(super) fn guarded_regions(body: &[Stmt]) -> Vec<TextRange> {
    filter_map_over_stmts(body, guarded_arms)
        .into_iter()
        .chain(filter_map_over_exprs(body, Descent::Into, guarded_spans))
        .flatten()
        .collect()
}

/// The span of each arm `stmt` guards, `None` for a statement that
/// guards none. An `if` and a `match` guard no arm here. A `while`
/// test re-runs on each pass and an `except` clause's type expression
/// runs only once a raise reaches it, so both join the arm bodies.
fn guarded_arms(stmt: &Stmt) -> Option<Vec<TextRange>> {
    let deferred = match stmt {
        Stmt::For(_) | Stmt::FunctionDef(_) | Stmt::With(_) => Vec::new(),
        Stmt::Try(node) => node
            .handlers
            .iter()
            .filter_map(|ExceptHandler::ExceptHandler(handler)| {
                handler.type_.as_deref().map(Ranged::range)
            })
            .collect(),
        Stmt::While(node) => vec![node.test.range()],
        _ => return None,
    };
    Some(
        sub_bodies(stmt)
            .into_iter()
            .map(|(body, _)| blocks_span(body))
            .chain(deferred)
            .collect(),
    )
}

/// The spans of `expr` that run per item or on a later call, `None` for
/// an expression that defers nothing. A lambda defers its body alone,
/// leaving its parameter defaults to the enclosing scope, and a
/// comprehension's first iterable runs there too, so both sit outside
/// the spans returned.
fn guarded_spans(expr: &Expr) -> Option<Vec<TextRange>> {
    let generators = match expr {
        Expr::DictComp(ExprDictComp { generators, .. })
        | Expr::Generator(ExprGenerator { generators, .. })
        | Expr::ListComp(ExprListComp { generators, .. })
        | Expr::SetComp(ExprSetComp { generators, .. }) => generators,
        Expr::Lambda(lambda) => return Some(vec![lambda.body.range()]),
        _ => return None,
    };
    let Some(first) = generators.first() else {
        unreachable!("invariant: a comprehension always carries one `for` clause");
    };
    let eager = first.iter.range();
    Some(
        [
            TextRange::new(expr.start(), eager.start()),
            TextRange::new(eager.end(), expr.end()),
        ]
        .into_iter()
        .filter(|span| !span.is_empty())
        .collect(),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, parse};

    #[rstest]
    #[case::loop_body("def f():\n    for i in xs:\n        g(i)\n", 1)]
    #[case::with_body("def f():\n    with open(p) as h:\n        g(h)\n", 1)]
    #[case::loop_else(
        "def f():\n    for i in xs:\n        g(i)\n    else:\n        h()\n",
        2
    )]
    #[case::while_test_and_body("def f():\n    while go:\n        g()\n", 2)]
    #[case::nested_def("def f():\n    def inner():\n        g()\n    return inner\n", 1)]
    #[case::lambda_body("def f():\n    return lambda: g()\n", 1)]
    #[case::dict_comprehension("def f():\n    return {k: v for k in xs}\n", 2)]
    #[case::set_comprehension("def f():\n    return {g(x) for x in xs}\n", 2)]
    #[case::generator("def f():\n    return sum(g(x) for x in xs)\n", 1)]
    #[case::if_branch("def f():\n    if go:\n        g()\n", 0)]
    #[case::match_case("def f():\n    match go:\n        case 1:\n            g()\n", 0)]
    #[case::plain_return("def f():\n    return 1\n", 0)]
    fn guarded_regions_counts_the_spans_a_shape_opens(#[case] src: &str, #[case] regions: usize) {
        let source = parse(src);
        assert_eq!(guarded_regions(&first_def(&source).body).len(), regions);
    }

    #[test]
    fn guarded_regions_reaches_a_comprehension_inside_a_replacement_field() {
        let source = parse("def f():\n    return f\"{[g(x) for x in xs]}\"\n");
        assert_eq!(guarded_regions(&first_def(&source).body).len(), 2);
    }

    #[test]
    fn guarded_regions_splits_a_comprehension_around_its_first_iterable() {
        let source = parse("def f():\n    return [g(x) for x in xs]\n");
        let regions = guarded_regions(&first_def(&source).body);
        let text = source.text();

        assert_eq!(regions.len(), 2);
        assert_eq!(&text[regions[0]], "[g(x) for x in ");
        assert_eq!(&text[regions[1]], "]");
    }

    #[test]
    fn guarded_regions_splits_a_try_into_its_arms_and_clause() {
        let source = parse(
            "def f():\n    try:\n        a()\n    except E:\n        b()\n    else:\n        c()\n    finally:\n        d()\n",
        );
        assert_eq!(guarded_regions(&first_def(&source).body).len(), 5);
    }

    #[test]
    fn guarded_spans_leaves_a_lambda_parameter_default_outside() {
        let source = parse("def f():\n    return lambda a=sep: g(a)\n");
        let regions = guarded_regions(&first_def(&source).body);

        assert_eq!(regions.len(), 1);
        assert_eq!(&source.text()[regions[0]], "g(a)");
    }
}

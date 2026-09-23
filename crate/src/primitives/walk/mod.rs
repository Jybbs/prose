//! Statement- and expression-tree probes over a module body.

mod parented;

pub(crate) use parented::{
    Descent, ParentedCollector, ParentedProbe, filter_map_over_parented_exprs,
    walk_parented_arguments, walk_parented_expr, walk_parented_exprs,
};

use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    statement_visitor::{self, StatementVisitor},
    visitor::{
        self, Visitor,
        source_order::{self, SourceOrderVisitor},
    },
};

/// Runs a caller's function on every annotation the walk reaches,
/// leaving the annotation's own subtree unvisited.
struct AnnotationProbe<F> {
    run: F,
}

impl<'src, F: FnMut(&Expr)> Visitor<'src> for AnnotationProbe<F> {
    fn visit_annotation(&mut self, annotation: &'src Expr) {
        (self.run)(annotation);
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        walk_stmt(self, stmt);
    }
}

/// The visitor behind [`any_over_expr_within`], which stops at the first
/// expression satisfying `hit`.
struct AnyExprProbe<F> {
    found: bool,
    hit: F,
    interpolations: Descent,
}

impl<'src, F: FnMut(&Expr) -> bool> SourceOrderVisitor<'src> for AnyExprProbe<F> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        if self.found {
            return;
        }
        if (self.hit)(expr) {
            self.found = true;
        } else {
            source_order::walk_expr(self, expr);
        }
    }

    fn visit_interpolated_string_element(&mut self, element: &'src InterpolatedStringElement) {
        if matches!(self.interpolations, Descent::Into) {
            source_order::walk_interpolated_string_element(self, element);
        }
    }
}

struct AnyProbe<F> {
    found: bool,
    hit: F,
}

impl<'src, F: FnMut(&Stmt) -> bool> StatementVisitor<'src> for AnyProbe<F> {
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        if self.found {
            return;
        }
        if (self.hit)(stmt) {
            self.found = true;
        } else {
            statement_visitor::walk_stmt(self, stmt);
        }
    }
}

struct Collector<F, T> {
    found: Vec<T>,
    probe: F,
}

impl<'src, F: FnMut(&'src Stmt) -> Option<T>, T> StatementVisitor<'src> for Collector<F, T> {
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.found.extend((self.probe)(stmt));
        statement_visitor::walk_stmt(self, stmt);
    }
}

struct ExprCollector<F, T> {
    found: Vec<T>,
    interpolations: Descent,
    probe: F,
}

impl<'src, F: FnMut(&Expr) -> Option<T>, T> SourceOrderVisitor<'src> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        self.found.extend((self.probe)(expr));
        source_order::walk_expr(self, expr);
    }

    fn visit_interpolated_string_element(&mut self, element: &'src InterpolatedStringElement) {
        if matches!(self.interpolations, Descent::Into) {
            source_order::walk_interpolated_string_element(self, element);
        }
    }
}

/// True when `hit` holds for `expr` or any expression beneath it, and
/// the walk stops at the first match. `interpolations` decides whether
/// the walk reads the interior of an f-string or t-string replacement
/// field.
pub(crate) fn any_over_expr_within(
    expr: &Expr,
    interpolations: Descent,
    hit: impl FnMut(&Expr) -> bool,
) -> bool {
    let mut probe = AnyExprProbe {
        found: false,
        hit,
        interpolations,
    };
    probe.visit_expr(expr);
    probe.found
}

/// True when any statement in `body` satisfies `hit`, descending through
/// every compound body including nested `def` and `class` scopes and
/// stopping at the first match.
pub(crate) fn any_over_stmts(body: &[Stmt], hit: impl FnMut(&Stmt) -> bool) -> bool {
    let mut probe = AnyProbe { found: false, hit };
    probe.visit_body(body);
    probe.found
}

/// Every `Some` that `probe` returns over each expression in `body` in
/// source order, descending through every compound body including
/// nested `def` and `class` scopes. `interpolations` decides whether the
/// walk reads the interior of an f-string or t-string replacement field.
pub(crate) fn filter_map_over_exprs<T>(
    body: &[Stmt],
    interpolations: Descent,
    probe: impl FnMut(&Expr) -> Option<T>,
) -> Vec<T> {
    let mut collector = ExprCollector {
        found: Vec::new(),
        interpolations,
        probe,
    };
    collector.visit_body(body);
    collector.found
}

/// Every `Some` that `probe` returns over `body`, descending through
/// every compound body including nested `def` and `class` scopes.
pub(crate) fn filter_map_over_stmts<'src, T>(
    body: &'src [Stmt],
    probe: impl FnMut(&'src Stmt) -> Option<T>,
) -> Vec<T> {
    let mut collector = Collector {
        found: Vec::new(),
        probe,
    };
    collector.visit_body(body);
    collector.found
}

/// Runs `run` over every annotation expression in `body`, covering an
/// annotated assignment's annotation plus each parameter and return
/// annotation of a function definition.
pub(crate) fn for_each_annotation(body: &[Stmt], run: impl FnMut(&Expr)) {
    let mut probe = AnnotationProbe { run };
    probe.visit_body(body);
}

/// Walks `stmt`'s children the way `visitor::walk_stmt` does, visiting
/// each elif clause's test once. The upstream walk visits that test
/// directly and then again through `walk_elif_else_clause`, so an `if`
/// statement walks its parts here and every other statement walks
/// upstream.
pub(crate) fn walk_stmt<'src, V: Visitor<'src> + ?Sized>(visitor: &mut V, stmt: &'src Stmt) {
    let Stmt::If(stmt_if) = stmt else {
        visitor::walk_stmt(visitor, stmt);
        return;
    };
    visitor.visit_expr(&stmt_if.test);
    visitor.visit_body(&stmt_if.body);
    for clause in &stmt_if.elif_else_clauses {
        visitor::walk_elif_else_clause(visitor, clause);
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

    /// The name of every `def` in `src`, in walk order.
    fn def_names(src: &str) -> Vec<String> {
        filter_map_over_stmts(&parse(src).ast().body, |stmt| {
            Some(stmt.as_function_def_stmt()?.name.to_string())
        })
    }

    /// The entry count of every dict literal in `src`, in walk order.
    fn dict_sizes(src: &str) -> Vec<usize> {
        filter_map_over_exprs(&parse(src).ast().body, Descent::Over, |expr| {
            Some(expr.as_dict_expr()?.len())
        })
    }

    fn has_pass(src: &str) -> bool {
        any_over_stmts(&parse(src).ast().body, |stmt| matches!(stmt, Stmt::Pass(_)))
    }

    #[rstest]
    #[case::a_dict_outside_any_field("[{'a': 1}, f\"{x}\"]", Descent::Over, true)]
    #[case::a_field_read_through("[f\"{ {'a': 1} }\"]", Descent::Into, true)]
    #[case::a_field_left_unwalked("[f\"{ {'a': 1} }\"]", Descent::Over, false)]
    fn any_over_expr_within_reads_a_replacement_field_per_its_descent(
        #[case] src: &str,
        #[case] interpolations: Descent,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(
            any_over_expr_within(first_expr(&source), interpolations, Expr::is_dict_expr),
            expected,
        );
    }

    #[test]
    fn any_over_expr_within_stops_at_the_first_match() {
        let source = parse("[a, b]\n");
        let mut seen = 0;
        let found = any_over_expr_within(first_expr(&source), Descent::Over, |expr| {
            seen += 1;
            expr.is_name_expr()
        });
        assert!(found);
        assert_eq!(seen, 2, "the walk stops at `a` rather than visiting `b`");
    }

    #[test]
    fn any_over_stmts_descends_into_a_nested_scope() {
        assert!(has_pass(indoc! {"
            class C:
                def f():
                    if cond:
                        pass
        "}));
    }

    #[test]
    fn any_over_stmts_is_false_when_nothing_matches() {
        assert!(!has_pass("x = 1\n"));
    }

    #[test]
    fn any_over_stmts_stops_at_the_first_match() {
        let mut seen = 0;
        let found = any_over_stmts(&parse("pass\npass\n").ast().body, |stmt| {
            seen += 1;
            matches!(stmt, Stmt::Pass(_))
        });
        assert!(found);
        assert_eq!(
            seen, 1,
            "the walk stops rather than visiting the second pass"
        );
    }

    #[test]
    fn filter_map_over_exprs_visits_an_elif_test_once() {
        assert_eq!(
            dict_sizes("if a:\n    pass\nelif {'b': 1}:\n    pass\n"),
            vec![1]
        );
    }

    #[test]
    fn filter_map_over_exprs_collects_through_a_nested_scope() {
        let sizes = dict_sizes(indoc! {r#"
            class C:
                def f():
                    outer = {"a": 1, "b": 2}
                    inner = {"c": 3}
        "#});
        assert_eq!(sizes, vec![2, 1], "the walk reaches a nested scope");
    }

    #[test]
    fn filter_map_over_exprs_skips_a_replacement_field() {
        let sizes = dict_sizes(indoc! {r#"
            plain = {"a": 1}
            label = f"{ {'b': 2, 'c': 3} }"
        "#});
        assert_eq!(sizes, vec![1], "the interpolated dict goes unvisited");
    }

    #[test]
    fn filter_map_over_stmts_collects_through_a_nested_scope() {
        let names = def_names(indoc! {"
            class C:
                def outer():
                    def inner():
                        pass
        "});
        assert_eq!(
            names,
            vec!["outer", "inner"],
            "the walk does not stop at outer"
        );
    }

    #[test]
    fn filter_map_over_stmts_is_empty_when_nothing_matches() {
        assert!(def_names("x = 1\n").is_empty());
    }
}

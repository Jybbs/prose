//! Statement- and expression-tree probes over a module body.

mod parented;

pub(crate) use parented::{
    Descent, ParentedCollector, ParentedProbe, filter_map_over_parented_exprs,
    walk_parented_arguments, walk_parented_expr, walk_parented_exprs,
};

use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    statement_visitor::{self, StatementVisitor},
    visitor::{self, Visitor, walk_expr},
};

/// Carries a caller's function to every annotation the walk reaches,
/// leaving the annotation's own subtree unvisited so the function reads
/// each one whole.
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

impl<'src, F: FnMut(&Expr) -> Option<T>, T> Visitor<'src> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        self.found.extend((self.probe)(expr));
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        walk_stmt(self, stmt);
    }

    fn visit_interpolated_string_element(&mut self, element: &'src InterpolatedStringElement) {
        if matches!(self.interpolations, Descent::Into) {
            visitor::walk_interpolated_string_element(self, element);
        }
    }
}

/// True when any statement in `body` satisfies `hit`, descending through
/// every compound body including nested `def` and `class` scopes and
/// stopping at the first match.
pub(crate) fn any_over_stmts(body: &[Stmt], hit: impl FnMut(&Stmt) -> bool) -> bool {
    let mut probe = AnyProbe { found: false, hit };
    probe.visit_body(body);
    probe.found
}

/// Every `Some` that `probe` returns over each expression in `body`,
/// descending through every compound body including nested `def` and
/// `class` scopes. `interpolations` decides whether the walk reads the
/// interior of an f-string or t-string replacement field.
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

    use super::*;
    use crate::testing::parse;

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

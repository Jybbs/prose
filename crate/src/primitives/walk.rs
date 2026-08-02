//! Statement- and expression-tree probes over a module body.

use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, InterpolatedStringElement, ModModule, Stmt,
    statement_visitor::{self, StatementVisitor},
    visitor::{Visitor, walk_arguments, walk_expr, walk_interpolated_string_element, walk_stmt},
};

/// Whether a walk reaches inside an f-string or t-string replacement
/// field.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Interpolations {
    Opaque,
    Walked,
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

impl<'src, F: FnMut(&Stmt) -> Option<T>, T> StatementVisitor<'src> for Collector<F, T> {
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.found.extend((self.probe)(stmt));
        statement_visitor::walk_stmt(self, stmt);
    }
}

struct ParentedCollector<'src, F, T> {
    interpolations: Interpolations,
    found: Vec<T>,
    parents: Vec<AnyNodeRef<'src>>,
    probe: F,
}

impl<'src, F: FnMut(&'src Expr, AnyNodeRef<'src>) -> Option<T>, T> Visitor<'src>
    for ParentedCollector<'src, F, T>
{
    fn visit_arguments(&mut self, arguments: &'src Arguments) {
        self.parents.push(arguments.into());
        walk_arguments(self, arguments);
        self.parents.pop();
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        let parent = *self.parents.last().expect("seeded with the module node");
        self.found.extend((self.probe)(expr, parent));
        self.parents.push(expr.into());
        walk_expr(self, expr);
        self.parents.pop();
    }

    fn visit_interpolated_string_element(&mut self, element: &'src InterpolatedStringElement) {
        if self.interpolations == Interpolations::Walked {
            walk_interpolated_string_element(self, element);
        }
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.parents.push(stmt.into());
        walk_stmt(self, stmt);
        self.parents.pop();
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

/// Every `Some` that `probe` returns over each expression under
/// `module`, descending through every compound body including nested
/// `def` and `class` scopes. An expression inside an f-string or
/// t-string replacement field is not visited.
pub(crate) fn filter_map_over_exprs<'src, T>(
    module: &'src ModModule,
    mut probe: impl FnMut(&'src Expr) -> Option<T>,
) -> Vec<T> {
    filter_map_over_parented_exprs(module, Interpolations::Opaque, |expr, _| probe(expr))
}

/// Every `Some` that `probe` returns over each expression under
/// `module`, paired with the node enclosing it. The module itself is
/// the outermost parent, and a call's argument resolves against the
/// `Arguments` node so a parenthesis-aware range recovers its pair.
/// `interpolations` decides whether the walk reaches inside an f-string or
/// t-string replacement field.
pub(crate) fn filter_map_over_parented_exprs<'src, T>(
    module: &'src ModModule,
    interpolations: Interpolations,
    probe: impl FnMut(&'src Expr, AnyNodeRef<'src>) -> Option<T>,
) -> Vec<T> {
    let mut collector = ParentedCollector {
        interpolations,
        found: Vec::new(),
        parents: vec![module.into()],
        probe,
    };
    collector.visit_body(&module.body);
    collector.found
}

/// Every `Some` that `probe` returns over `body`, descending through
/// every compound body including nested `def` and `class` scopes.
pub(crate) fn filter_map_over_stmts<T>(
    body: &[Stmt],
    probe: impl FnMut(&Stmt) -> Option<T>,
) -> Vec<T> {
    let mut collector = Collector {
        found: Vec::new(),
        probe,
    };
    collector.visit_body(body);
    collector.found
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use ruff_text_size::Ranged;

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
        filter_map_over_exprs(parse(src).ast(), |expr| Some(expr.as_dict_expr()?.len()))
    }

    fn has_pass(src: &str) -> bool {
        any_over_stmts(&parse(src).ast().body, |stmt| matches!(stmt, Stmt::Pass(_)))
    }

    /// The source text of the parent reported for the `Name` bound to
    /// `id` in `src`.
    fn parent_of_name(src: &str, id: &str) -> String {
        let source = parse(src);
        let parents =
            filter_map_over_parented_exprs(source.ast(), Interpolations::Opaque, |expr, parent| {
                expr.as_name_expr()
                    .filter(|name| name.id == *id)
                    .map(|_| parent.range())
            });
        source
            .slice(*parents.first().expect("the name is present"))
            .to_owned()
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
    fn filter_map_over_parented_exprs_pairs_a_call_argument_with_its_arguments_node() {
        assert_eq!(parent_of_name("f(a)\n", "a"), "(a)");
    }

    #[test]
    fn filter_map_over_parented_exprs_pairs_an_operand_with_its_expression() {
        assert_eq!(parent_of_name("y = a + b\n", "a"), "a + b");
    }

    #[test]
    fn filter_map_over_parented_exprs_skips_a_replacement_field_when_opaque() {
        let source = parse("print(a, f\"{b}\")\n");
        let names =
            filter_map_over_parented_exprs(source.ast(), Interpolations::Opaque, |expr, _| {
                Some(expr.as_name_expr()?.id.to_string())
            });
        assert_eq!(
            names,
            vec!["print", "a"],
            "the interpolated name goes unvisited",
        );
    }

    #[test]
    fn filter_map_over_parented_exprs_walks_a_replacement_field_when_asked() {
        let source = parse("print(a, f\"{b}\")\n");
        let names =
            filter_map_over_parented_exprs(source.ast(), Interpolations::Walked, |expr, _| {
                Some(expr.as_name_expr()?.id.to_string())
            });
        assert_eq!(names, vec!["print", "a", "b"]);
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

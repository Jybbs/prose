//! Statement- and expression-tree probes over a module body, plus the
//! parent-tracking expression walk a rule drives when it needs the node
//! enclosing each expression.

use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, InterpolatedStringElement, ModModule, Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
    visitor::{self, Visitor, walk_expr},
};

/// Whether a parent-tracking walk descends into the expression its probe
/// just read.
pub(crate) enum Descent {
    /// Visit the expression's own children next.
    Into,
    /// Leave the children unvisited.
    Over,
}

/// Reads each expression of a module alongside the node enclosing it.
pub(crate) trait ParentedProbe<'src> {
    fn probe(&mut self, expr: &'src Expr, parent: AnyNodeRef<'src>) -> Descent;
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
            walk_stmt(self, stmt);
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
        walk_stmt(self, stmt);
    }
}

struct ExprCollector<F, T> {
    found: Vec<T>,
    probe: F,
}

impl<'src, F: FnMut(&Expr) -> Option<T>, T> Visitor<'src> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        self.found.extend((self.probe)(expr));
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'src InterpolatedStringElement) {}
}

struct ParentedWalk<'src, 'probe, P> {
    parents: Vec<AnyNodeRef<'src>>,
    probe: &'probe mut P,
}

impl<'src, P: ParentedProbe<'src>> Visitor<'src> for ParentedWalk<'src, '_, P> {
    fn visit_arguments(&mut self, arguments: &'src Arguments) {
        self.parents.push(arguments.into());
        visitor::walk_arguments(self, arguments);
        self.parents.pop();
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        let parent = *self.parents.last().expect("seeded with the module node");
        if matches!(self.probe.probe(expr, parent), Descent::Over) {
            return;
        }
        self.parents.push(expr.into());
        walk_expr(self, expr);
        self.parents.pop();
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.parents.push(stmt.into());
        visitor::walk_stmt(self, stmt);
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

/// Every `Some` that `probe` returns over each expression in `body`,
/// descending through every compound body including nested `def` and
/// `class` scopes. An expression inside an f-string or t-string
/// replacement field is not visited.
pub(crate) fn filter_map_over_exprs<T>(
    body: &[Stmt],
    probe: impl FnMut(&Expr) -> Option<T>,
) -> Vec<T> {
    let mut collector = ExprCollector {
        found: Vec::new(),
        probe,
    };
    collector.visit_body(body);
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

/// Walks every expression in `module`, handing each to `probe` with the
/// node enclosing it and descending unless the probe reports `Over`. A
/// call argument names its `Arguments` list rather than the call, so a
/// sole argument's enclosing range stops short of the call's own
/// parentheses.
pub(crate) fn walk_parented_exprs<'src>(
    module: &'src ModModule,
    probe: &mut impl ParentedProbe<'src>,
) {
    ParentedWalk {
        parents: vec![AnyNodeRef::from(module)],
        probe,
    }
    .visit_body(&module.body);
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::{source::Source, testing::parse};

    /// Records each expression's text paired with whether the argument
    /// list encloses it, stepping over the expression matching `halt`.
    struct Trace<'a> {
        halt: &'a str,
        seen: Vec<(&'a str, bool)>,
        source: &'a Source,
    }

    impl<'a> ParentedProbe<'a> for Trace<'a> {
        fn probe(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>) -> Descent {
            let text = self.source.slice(expr);
            self.seen
                .push((text, matches!(parent, AnyNodeRef::Arguments(_))));
            if text == self.halt {
                Descent::Over
            } else {
                Descent::Into
            }
        }
    }

    /// The name of every `def` in `src`, in walk order.
    fn def_names(src: &str) -> Vec<String> {
        filter_map_over_stmts(&parse(src).ast().body, |stmt| {
            Some(stmt.as_function_def_stmt()?.name.to_string())
        })
    }

    /// The entry count of every dict literal in `src`, in walk order.
    fn dict_sizes(src: &str) -> Vec<usize> {
        filter_map_over_exprs(&parse(src).ast().body, |expr| {
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

    #[rstest]
    #[case::descends_naming_the_argument_list(
        "",
        vec![("f(a)", false), ("f", false), ("a", true)],
    )]
    #[case::steps_over_the_members_it_covered("f(a)", vec![("f(a)", false)])]
    fn walk_parented_exprs_names_each_parent_and_honors_the_descent(
        #[case] halt: &str,
        #[case] expected: Vec<(&str, bool)>,
    ) {
        let source = parse("f(a)\n");
        let mut trace = Trace {
            halt,
            seen: Vec::new(),
            source: &source,
        };
        walk_parented_exprs(source.ast(), &mut trace);
        assert_eq!(trace.seen, expected);
    }
}

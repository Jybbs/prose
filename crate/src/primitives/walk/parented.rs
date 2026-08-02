//! The parent-tracking expression walk a rule drives when it needs the
//! node enclosing each expression.

use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, ModModule, Stmt,
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

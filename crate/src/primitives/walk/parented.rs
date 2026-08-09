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

/// Reads each expression of a module alongside the node enclosing it
/// and the full ancestor chain, outermost first.
pub(crate) trait ParentedProbe<'src> {
    fn probe(
        &mut self,
        expr: &'src Expr,
        parent: AnyNodeRef<'src>,
        ancestors: &[AnyNodeRef<'src>],
    ) -> Descent;
}

struct ParentedCollector<F, T> {
    found: Vec<T>,
    interpolations: Descent,
    probe: F,
}

impl<'src, F: FnMut(&'src Expr, AnyNodeRef<'src>) -> Option<T>, T> ParentedProbe<'src>
    for ParentedCollector<F, T>
{
    fn probe(
        &mut self,
        expr: &'src Expr,
        parent: AnyNodeRef<'src>,
        _: &[AnyNodeRef<'src>],
    ) -> Descent {
        if is_interpolated_string(expr) && matches!(self.interpolations, Descent::Over) {
            return Descent::Over;
        }
        self.found.extend((self.probe)(expr, parent));
        Descent::Into
    }
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
        if matches!(self.probe.probe(expr, parent, &self.parents), Descent::Over) {
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

/// Every `Some` that `probe` returns over each expression in `module`,
/// each read alongside the node enclosing it. `interpolations` decides
/// whether the walk reads the interior of a replacement field.
pub(crate) fn filter_map_over_parented_exprs<'src, T>(
    module: &'src ModModule,
    interpolations: Descent,
    probe: impl FnMut(&'src Expr, AnyNodeRef<'src>) -> Option<T>,
) -> Vec<T> {
    let mut collector = ParentedCollector {
        found: Vec::new(),
        interpolations,
        probe,
    };
    walk_parented_exprs(module, &mut collector);
    collector.found
}

/// True for an f-string or t-string, the expression a probe reports
/// `Descent::Over` on to leave every replacement field inside it the
/// shape its author gave it.
pub(crate) const fn is_interpolated_string(expr: &Expr) -> bool {
    matches!(expr, Expr::FString(_) | Expr::TString(_))
}

/// Walks every expression in `module`, handing each to `probe` with the
/// node enclosing it and the ancestor chain above it, descending unless
/// the probe reports `Over`. A call argument names its `Arguments` list
/// rather than the call, so a sole argument's enclosing range stops
/// short of the call's own parentheses.
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
        fn probe(
            &mut self,
            expr: &'a Expr,
            parent: AnyNodeRef<'a>,
            _: &[AnyNodeRef<'a>],
        ) -> Descent {
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

    #[test]
    fn filter_map_over_parented_exprs_hands_each_expression_its_parent() {
        let source = parse("f(a)\n");
        let enclosed =
            filter_map_over_parented_exprs(source.ast(), Descent::Over, |expr, parent| {
                matches!(expr, Expr::Name(_)).then(|| matches!(parent, AnyNodeRef::Arguments(_)))
            });
        assert_eq!(enclosed, vec![false, true], "the argument names its list");
    }

    #[rstest]
    #[case(Descent::Over, vec![1])]
    #[case(Descent::Into, vec![1, 2])]
    fn filter_map_over_parented_exprs_reads_a_replacement_field_on_request(
        #[case] interpolations: Descent,
        #[case] expected: Vec<usize>,
    ) {
        let source = parse("plain = {\"a\": 1}\nlabel = f\"{ {'b': 2, 'c': 3} }\"\n");
        let sizes = filter_map_over_parented_exprs(source.ast(), interpolations, |expr, _| {
            expr.as_dict_expr().map(|dict| dict.items.len())
        });
        assert_eq!(sizes, expected);
    }

    #[rstest]
    #[case::f_string("f\"{a}\"\n", true)]
    #[case::t_string("t\"{a}\"\n", true)]
    #[case::plain_string("\"a\"\n", false)]
    #[case::name("a\n", false)]
    fn is_interpolated_string_names_the_two_replacement_field_carriers(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = source.ast().body[0]
            .as_expr_stmt()
            .expect("the fixture is one expression statement");
        assert_eq!(is_interpolated_string(&expr.value), expected);
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

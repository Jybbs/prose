//! The parent-tracking expression walk, handing a rule the node
//! enclosing each expression in the order the source writes them.

use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, ExprCall, ModModule, Stmt,
    visitor::source_order::{self, SourceOrderVisitor, TraversalSignal},
};

/// Whether a walk reads the interior of an f-string or t-string
/// replacement field.
#[derive(Clone, Copy)]
pub(crate) enum Interpolations {
    /// Walk each replacement field's expression.
    Read,
    /// Leave every replacement field unwalked.
    Skip,
}

/// Reads each expression of a module alongside the node enclosing it
/// and the full ancestor chain, outermost first.
pub(crate) trait ParentedProbe<'src> {
    /// Whether the walk reads the interior of a replacement field, a
    /// probe set to `Skip` leaving every f-string and t-string it
    /// reaches unwalked whatever it reports on the string itself.
    const INTERPOLATIONS: Interpolations = Interpolations::Read;

    fn probe(
        &mut self,
        expr: &'src Expr,
        parent: AnyNodeRef<'src>,
        ancestors: &[AnyNodeRef<'src>],
    ) -> TraversalSignal;
}

/// Collects each `Some` that `probe` returns, descending past a hit
/// per `on_hit`, so `TraversalSignal::Skip` keeps the outermost hits alone.
pub(crate) struct ParentedCollector<F, T> {
    pub(crate) found: Vec<T>,
    interpolations: Interpolations,
    on_hit: TraversalSignal,
    probe: F,
}

impl<F, T> ParentedCollector<F, T> {
    pub(crate) fn new(interpolations: Interpolations, on_hit: TraversalSignal, probe: F) -> Self {
        Self {
            found: Vec::new(),
            interpolations,
            on_hit,
            probe,
        }
    }
}

impl<'src, F: FnMut(&'src Expr, AnyNodeRef<'src>) -> Option<T>, T> ParentedProbe<'src>
    for ParentedCollector<F, T>
{
    fn probe(
        &mut self,
        expr: &'src Expr,
        parent: AnyNodeRef<'src>,
        _: &[AnyNodeRef<'src>],
    ) -> TraversalSignal {
        let signal = match (self.probe)(expr, parent) {
            Some(found) => {
                self.found.push(found);
                self.on_hit
            }
            None => TraversalSignal::Traverse,
        };
        if is_interpolated_string(expr) && matches!(self.interpolations, Interpolations::Skip) {
            return TraversalSignal::Skip;
        }
        signal
    }
}

struct ParentedWalk<'src, 'probe, P> {
    parents: Vec<AnyNodeRef<'src>>,
    probe: &'probe mut P,
}

impl<'src, P: ParentedProbe<'src>> SourceOrderVisitor<'src> for ParentedWalk<'src, '_, P> {
    fn visit_arguments(&mut self, arguments: &'src Arguments) {
        self.parents.push(arguments.into());
        source_order::walk_arguments(self, arguments);
        self.parents.pop();
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        let parent = *self.parents.last().expect("seeded with the module node");
        if !self.probe.probe(expr, parent, &self.parents).is_traverse()
            || (matches!(P::INTERPOLATIONS, Interpolations::Skip) && is_interpolated_string(expr))
        {
            return;
        }
        self.parents.push(expr.into());
        source_order::walk_expr(self, expr);
        self.parents.pop();
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.parents.push(stmt.into());
        source_order::walk_stmt(self, stmt);
        self.parents.pop();
    }
}

/// True for an f-string or t-string, the expression a probe set to
/// `Interpolations::Skip` leaves unwalked.
const fn is_interpolated_string(expr: &Expr) -> bool {
    matches!(expr, Expr::FString(_) | Expr::TString(_))
}

/// Every `Some` that `probe` returns over each expression in `module`,
/// each read alongside the node enclosing it. `interpolations` decides
/// whether the walk reads the interior of a replacement field.
pub(crate) fn filter_map_over_parented_exprs<'src, T>(
    module: &'src ModModule,
    interpolations: Interpolations,
    probe: impl FnMut(&'src Expr, AnyNodeRef<'src>) -> Option<T>,
) -> Vec<T> {
    let mut collector = ParentedCollector::new(interpolations, TraversalSignal::Traverse, probe);
    walk_parented_exprs(module, &mut collector);
    collector.found
}

/// Walks every expression inside `call`'s argument list the way
/// [`walk_parented_exprs`] walks a module, the list named as the node
/// enclosing each top-level argument.
pub(crate) fn walk_parented_arguments<'src>(
    call: &'src ExprCall,
    probe: &mut impl ParentedProbe<'src>,
) {
    ParentedWalk {
        parents: vec![AnyNodeRef::from(call)],
        probe,
    }
    .visit_arguments(&call.arguments);
}

/// Walks `expr` and every expression beneath it the way
/// [`walk_parented_exprs`] walks a module, `parent` naming the node
/// enclosing `expr`.
pub(crate) fn walk_parented_expr<'src>(
    expr: &'src Expr,
    parent: AnyNodeRef<'src>,
    probe: &mut impl ParentedProbe<'src>,
) {
    ParentedWalk {
        parents: vec![parent],
        probe,
    }
    .visit_expr(expr);
}

/// Walks every expression in `module` in source order, handing each to
/// `probe` with the node enclosing it and the ancestor chain above it,
/// descending unless the probe reports `Skip`. A call argument names its
/// `Arguments` list rather than the call, so a sole argument's enclosing
/// range stops short of the call's own parentheses.
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
        ) -> TraversalSignal {
            let text = self.source.slice(expr);
            self.seen
                .push((text, matches!(parent, AnyNodeRef::Arguments(_))));
            if text == self.halt {
                TraversalSignal::Skip
            } else {
                TraversalSignal::Traverse
            }
        }
    }

    #[test]
    fn filter_map_over_parented_exprs_hands_each_expression_its_parent() {
        let source = parse("f(a)\n");
        let enclosed =
            filter_map_over_parented_exprs(source.ast(), Interpolations::Skip, |expr, parent| {
                matches!(expr, Expr::Name(_)).then(|| matches!(parent, AnyNodeRef::Arguments(_)))
            });
        assert_eq!(enclosed, vec![false, true], "the argument names its list");
    }

    #[rstest]
    #[case(Interpolations::Skip, vec![1])]
    #[case(Interpolations::Read, vec![1, 2])]
    fn filter_map_over_parented_exprs_reads_a_replacement_field_on_request(
        #[case] interpolations: Interpolations,
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
    fn walk_parented_exprs_names_each_parent_and_honors_the_signal(
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

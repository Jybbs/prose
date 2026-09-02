//! Parameter primitives shared by `alphabetize-siblings`, the
//! `unsorted-positionals` lint, `reflow-calls`, and the rules that read a
//! signature's receiver. The sort key drives the keyword-only sort and
//! docstring mirror, and the decorator predicate gates the rules that
//! must not reorder a positionally-bound signature.

use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, Expr, ParameterWithDefault, Parameters, StmtFunctionDef,
};

use crate::primitives::{decorator::decorator_arguments, effect::value_is_effectful};

/// Composite parameter sort key. Required parameters (no default)
/// sort before optional parameters (has default), each sub-group by
/// name. `self` and `cls` pin in place, as does a parameter whose
/// default binds an effectful value, since defaults evaluate in
/// signature order at `def` time and a reorder would swap their side
/// effects.
pub(crate) fn classify_param(p: &ParameterWithDefault) -> Option<(u8, &str)> {
    let name = p.name().as_str();
    if is_receiver_name(name) || p.default.as_deref().is_some_and(value_is_effectful) {
        return None;
    }
    Some((u8::from(p.default.is_some()), name))
}

/// The parameter holding a signature's leading positional slot, the
/// receiver a method binds. `None` where the leading slot is
/// keyword-only or variadic.
pub(crate) fn first_positional(parameters: &Parameters) -> Option<&ParameterWithDefault> {
    parameters.posonlyargs.first().or(parameters.args.first())
}

/// True for the two names a method's leading positional slot binds.
pub(crate) fn is_receiver_name(name: &str) -> bool {
    matches!(name, "cls" | "self")
}

/// The annotation and default sites of `param`, each beside the node
/// its parentheses recover against, the default carried by a
/// non-variadic slot alone.
pub(crate) fn parameter_sites<'a>(param: AnyParameterRef<'a>) -> Vec<(&'a Expr, AnyNodeRef<'a>)> {
    let inner = param.as_parameter();
    let mut sites: Vec<(&Expr, AnyNodeRef)> = Vec::new();
    if let Some(annotation) = inner.annotation.as_deref() {
        sites.push((annotation, inner.into()));
    }
    if let AnyParameterRef::NonVariadic(slot) = param
        && let Some(default) = slot.default.as_deref()
    {
        sites.push((default, slot.into()));
    }
    sites
}

/// True when sorting a function's positional-or-keyword parameters by
/// the sort key would change their order, ignoring positional-only and
/// `self` / `cls` parameters that hold their slot.
pub(crate) fn params_unsorted(params: &Parameters) -> bool {
    !params.args.iter().filter_map(classify_param).is_sorted()
}

/// True when any of `f`'s decorators is a `Call` carrying positional
/// arguments, signalling the decorator may bind values into the
/// signature by position.
pub(crate) fn pins_positional_params(f: &StmtFunctionDef) -> bool {
    f.decorator_list
        .iter()
        .any(|d| decorator_arguments(d).is_some_and(|a| !a.args.is_empty()))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, parse};

    #[rstest]
    #[case("def m(self): pass", Some("self"))]
    #[case("def m(self, /, x): pass", Some("self"))]
    #[case("def m(x, *rest): pass", Some("x"))]
    #[case("def m(*, self): pass", None)]
    #[case("def m(*args): pass", None)]
    #[case("def m(**kwargs): pass", None)]
    #[case("def m(): pass", None)]
    fn first_positional_reads_the_leading_slot(#[case] src: &str, #[case] expected: Option<&str>) {
        let source = parse(&format!("{src}\n"));
        let receiver = first_positional(&first_def(&source).parameters);
        assert_eq!(receiver.map(|p| p.name().as_str()), expected);
    }

    #[rstest]
    #[case("def f(b, a): pass\n", true)]
    #[case("def f(a, b): pass\n", false)]
    #[case("def f(a): pass\n", false)]
    #[case("def f(): pass\n", false)]
    #[case("def f(self, b, a): pass\n", true)]
    #[case("def f(cls, b, a): pass\n", true)]
    #[case("def f(b, a, /): pass\n", false)]
    #[case("def f(a, b, *, d, c): pass\n", false)]
    #[case("def f(x, /, b, a): pass\n", true)]
    #[case("def f(zebra, apple=1): pass\n", false)]
    #[case("def f(b=1, a=2): pass\n", true)]
    fn params_unsorted_tracks_only_the_positional_or_keyword_args(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        assert_eq!(params_unsorted(&first_def(&s).parameters), expected);
    }
}

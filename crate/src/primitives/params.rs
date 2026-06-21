//! Parameter-reorder primitives shared by `alphabetize`, the
//! `unsorted-parameters` lint, and `call-layout`. The sort key drives
//! the keyword-only sort and docstring mirror, and the decorator
//! predicate gates the rules that must not reorder a positionally-bound
//! signature.

use ruff_python_ast::{ParameterWithDefault, Parameters, StmtFunctionDef};

/// Composite parameter sort key. Required parameters (no default)
/// sort before optional parameters (has default), each sub-group by
/// name. `self` and `cls` pin in place.
pub(crate) fn classify_param(p: &ParameterWithDefault) -> Option<(u8, &str)> {
    let name = p.name().as_str();
    if matches!(name, "cls" | "self") {
        return None;
    }
    Some((u8::from(p.default.is_some()), name))
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
    f.decorator_list.iter().any(|d| {
        d.expression
            .as_call_expr()
            .is_some_and(|c| !c.arguments.args.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, parse};

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
    fn params_unsorted_tracks_only_the_positional_or_keyword_args(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        assert_eq!(params_unsorted(&first_def(&s).parameters), expected);
    }
}

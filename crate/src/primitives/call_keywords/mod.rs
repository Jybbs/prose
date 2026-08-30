//! Renders a call's arguments in keyword form for the rules that
//! reshape call sites. [`keyword_args`] names each argument when the
//! whole call can take keyword form, [`module_call_params`] maps
//! in-module call sites to the signature they bind, and
//! [`resolve_call_params`] resolves one call's callee against that map.

use std::borrow::Cow;

mod params;

pub(crate) use params::{module_call_params, resolve_call_params};

use itertools::Itertools;
use ruff_python_ast::{Expr, ExprCall, ParameterWithDefault, Parameters, Stmt};
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashMap;

use crate::{primitives::params::pins_positional_params, source::Source};

/// Every nameable argument of a call in source order, flagged when a
/// leading positional-only argument keeps its slot.
pub(crate) struct CallKeywords<'src> {
    pub(crate) args: Vec<KeywordArg<'src>>,
    pub(crate) has_posonly_prefix: bool,
}

/// The callee-offset lookup [`module_call_params`] returns, resolving a
/// call to the module function it binds.
pub(crate) type CallTargets<'src> = FxHashMap<TextSize, &'src Parameters>;

/// One argument of a call rendered as a `name=value` keyword binding.
pub(crate) struct KeywordArg<'src> {
    /// The bound parameter or keyword name, the key for the
    /// `all_unique` collision guard in `keyword_args`.
    pub(crate) name: &'src str,
    /// The `name=value` text, borrowed for a keyword already in that
    /// form and owned for a positional argument named from its parameter,
    /// a row-spanning positional keeping the grouping pair that holds
    /// its rows together.
    pub(crate) rendered: Cow<'src, str>,
    /// The offset the argument opens at in the source, the opening of
    /// that pair or the value's own start for a positional argument the
    /// rendering names.
    pub(crate) start: TextSize,
    /// The argument's value expression, the recursion point for a
    /// consumer that reshapes a nested call.
    pub(crate) value: &'src Expr,
}

/// Renders `call`'s arguments past any positional-only prefix as
/// keyword bindings, in source order. `params` carries the resolved
/// signature when the callee binds a module function, or `None` when
/// the callee is external. Returns `None` when an argument cannot take
/// keyword form: a positional argument without a resolved name, a `*`
/// or `**` unpacking, overflow past the named parameters, or a
/// duplicate key.
pub(crate) fn keyword_args<'src>(
    source: &'src Source,
    call: &'src ExprCall,
    params: Option<&'src Parameters>,
) -> Option<CallKeywords<'src>> {
    let positional = &call.arguments.args;
    let keywords = &call.arguments.keywords;
    if positional.iter().any(Expr::is_starred_expr) || keywords.iter().any(|kw| kw.arg.is_none()) {
        return None;
    }
    let posonly = params.map_or(0, |p| p.posonlyargs.len());
    let named_params: &[ParameterWithDefault] = params.map_or(&[], |p| &p.args);
    if positional.len() > posonly + named_params.len() {
        return None;
    }
    let args: Vec<KeywordArg> = positional
        .iter()
        .skip(posonly)
        .zip(named_params)
        .map(|(arg, param)| {
            let name = param.name().as_str();
            let range = source.spanning_paren_range(arg.into(), (&call.arguments).into());
            let value = source.slice(range);
            KeywordArg {
                name,
                rendered: Cow::Owned(if requires_grouping(arg) && range == arg.range() {
                    format!("{name}=({value})")
                } else {
                    format!("{name}={value}")
                }),
                start: range.start(),
                value: arg,
            }
        })
        .chain(keywords.iter().map(|kw| KeywordArg {
            name: kw.arg.as_deref().expect("`**` keyword excluded above"),
            rendered: Cow::Borrowed(source.slice(kw)),
            start: kw.start(),
            value: &kw.value,
        }))
        .collect();
    args.iter()
        .map(|arg| arg.name)
        .all_unique()
        .then_some(CallKeywords {
            args,
            has_posonly_prefix: !positional.is_empty() && posonly > 0,
        })
}

/// True where `reflow-calls`'s count trigger explodes `call`, meaning
/// every argument takes keyword form against the module function the
/// callee binds and no positional-only prefix pins the order. A call the
/// cap claims but cannot name stays inline, so a join or a one-row form
/// written around it stands rather than being reopened. Without a target
/// map the answer holds at true, the reading that never writes a form a
/// later explode would undo.
pub(crate) fn takes_keyword_form(
    source: &Source,
    call: &ExprCall,
    targets: Option<&CallTargets<'_>>,
) -> bool {
    targets.is_none_or(|targets| {
        keyword_args(source, call, resolve_call_params(call, targets))
            .is_some_and(|keywords| !keywords.has_posonly_prefix)
    })
}

/// True for the argument shapes whose source slice does not parse after
/// a `name=` prefix, a named expression, a `yield`, a `yield from`, and
/// a generator expression carrying no parentheses of its own.
fn requires_grouping(arg: &Expr) -> bool {
    matches!(arg, Expr::Named(_) | Expr::Yield(_) | Expr::YieldFrom(_))
        || arg.as_generator_expr().is_some_and(|g| !g.parenthesized)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_def, first_expr, parse};

    #[rstest]
    #[case("1", "x=1")]
    #[case("a if b else c", "x=a if b else c")]
    #[case("lambda: 1", "x=lambda: 1")]
    #[case("[n for n in items]", "x=[n for n in items]")]
    #[case("a for a in items", "x=(a for a in items)")]
    #[case("(a for a in items)", "x=(a for a in items)")]
    #[case("y := 1", "x=(y := 1)")]
    #[case("(y := 1)", "x=(y := 1)")]
    #[case("(a)", "x=a")]
    #[case("(\n    a\n    .b()\n)", "x=(\n    a\n    .b()\n)")]
    #[case("(\n    y := 1\n)", "x=(y := 1)")]
    fn a_positional_renders_in_a_form_the_keyword_slot_accepts(
        #[case] argument: &str,
        #[case] expected: &str,
    ) {
        let source = parse(&format!("f({argument})\n"));
        let callee = parse("def f(x): pass\n");
        let call = first_expr(&source)
            .as_call_expr()
            .expect("the statement is a call");
        let keywords = keyword_args(&source, call, Some(&first_def(&callee).parameters))
            .expect("a sole resolved positional takes keyword form");

        assert_eq!(keywords.args[0].rendered, expected);
    }
}

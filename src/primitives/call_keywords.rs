//! Renders a call's arguments in keyword form for `call-layout`, the
//! rule that reshapes call sites. [`keyword_args`] names each argument
//! when the whole call can take keyword form, [`module_call_params`]
//! maps in-module call sites to the signature they bind, and
//! [`callee_params`] resolves one call against that map.

use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use ruff_python_ast::{Expr, ExprCall, ParameterWithDefault, Parameters, Stmt};
use ruff_text_size::{Ranged, TextSize};

use crate::{primitives::params::pins_positional_params, source::Source};

/// Every nameable argument of a call in source order, flagged when a
/// leading positional-only argument keeps its slot.
pub(crate) struct CallKeywords<'src> {
    pub args: Vec<KeywordArg<'src>>,
    pub has_posonly_prefix: bool,
}

/// One argument of a call rendered as a `name=value` keyword binding.
pub(crate) struct KeywordArg<'src> {
    /// The bound parameter or keyword name, the key for the
    /// `all_unique` collision guard in `keyword_args`.
    name: &'src str,
    /// The `name=value` text, borrowed for a keyword already in that
    /// form and owned for a positional argument named from its parameter.
    pub rendered: Cow<'src, str>,
    /// The argument's value expression, the recursion point for a
    /// consumer that reshapes a nested call.
    pub value: &'src Expr,
}

/// Looks up the parameters `targets` resolves for `call`'s callee.
/// `None` when the callee is not a plain name or is not in the map.
pub(crate) fn callee_params<'src>(
    targets: &HashMap<TextSize, &'src Parameters>,
    call: &ExprCall,
) -> Option<&'src Parameters> {
    targets
        .get(&call.func.as_name_expr()?.range().start())
        .copied()
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
    if !positional.is_empty() && params.is_none() {
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
            KeywordArg {
                name,
                rendered: Cow::Owned(format!("{name}={}", source.slice(arg))),
                value: arg,
            }
        })
        .chain(keywords.iter().map(|kw| KeywordArg {
            name: kw.arg.as_deref().expect("`**` keyword excluded above"),
            rendered: Cow::Borrowed(source.slice(kw)),
            value: &kw.value,
        }))
        .collect();
    args.iter()
        .map(|arg| arg.name)
        .all_unique()
        .then_some(CallKeywords {
            has_posonly_prefix: positional.len().min(posonly) > 0,
            args,
        })
}

/// Maps each in-module call's callee offset to the parameters of the
/// top-level function it resolves to, over every function whose
/// decorators do not bind by position and whose name binds uniquely to
/// that one definition. Offsets come from `BindingAnalysis`, so a
/// shadowing local or aliased reference resolves elsewhere.
pub(crate) fn module_call_params(source: &Source) -> HashMap<TextSize, &Parameters> {
    let analysis = source.binding_analysis();
    source
        .ast()
        .body
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|&func| !pins_positional_params(func))
        .filter_map(|func| Some((analysis.module_function_reads(func.name.as_str())?, func)))
        .flat_map(|(reads, func)| reads.iter().map(move |&offset| (offset, &*func.parameters)))
        .collect()
}

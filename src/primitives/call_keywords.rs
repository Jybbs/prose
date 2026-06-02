//! Renders a call's arguments in keyword form for the rules that
//! reshape call sites. [`keyword_args`] names each argument when the
//! whole call can take keyword form, [`module_call_params`] maps
//! in-module call sites to the signature they bind, and
//! [`pins_positional_params`] flags a positional-binding decorator.

use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use ruff_python_ast::{Expr, ExprCall, ParameterWithDefault, Parameters, Stmt, StmtFunctionDef};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::source::Source;

/// Every nameable argument of a call in source order, paired with the
/// count of leading positional-only arguments that keep their slot.
pub(crate) struct CallKeywords<'src> {
    pub args: Vec<KeywordArg<'src>>,
    pub posonly_prefix: usize,
}

/// One argument of a call rendered as a `name=value` keyword binding.
pub(crate) struct KeywordArg<'src> {
    /// The argument's source extent, the edit block.
    pub block: TextRange,
    /// The bound parameter or keyword name, the alphabetization key.
    pub name: &'src str,
    /// The `name=value` text, borrowed for a keyword already in that
    /// form and owned for a positional argument named from its parameter.
    pub rendered: Cow<'src, str>,
    /// The argument's value expression, the recursion point for a
    /// consumer that reshapes a nested call.
    pub value: &'src Expr,
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
                block: arg.range(),
                name,
                rendered: Cow::Owned(format!("{name}={}", source.slice(arg))),
                value: arg,
            }
        })
        .chain(keywords.iter().map(|kw| KeywordArg {
            block: kw.range(),
            name: kw.arg.as_deref().expect("`**` keyword excluded above"),
            rendered: Cow::Borrowed(source.slice(kw)),
            value: &kw.value,
        }))
        .collect();
    args.iter()
        .map(|arg| arg.name)
        .all_unique()
        .then_some(CallKeywords {
            posonly_prefix: positional.len().min(posonly),
            args,
        })
}

/// Maps each in-module call's callee offset to the parameters of the
/// top-level function it resolves to, over every function `accept`
/// admits whose decorators do not bind by position and whose name binds
/// uniquely to that one definition. Offsets come from `BindingAnalysis`,
/// so a shadowing local or aliased reference resolves elsewhere.
pub(crate) fn module_call_params<'src>(
    source: &'src Source,
    mut accept: impl FnMut(&'src StmtFunctionDef) -> bool,
) -> HashMap<TextSize, &'src Parameters> {
    let analysis = source.binding_analysis();
    source
        .ast()
        .body
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|&func| !pins_positional_params(func) && accept(func))
        .filter_map(|func| Some((analysis.module_function_reads(func.name.as_str())?, func)))
        .flat_map(|(reads, func)| reads.iter().map(move |&offset| (offset, &*func.parameters)))
        .collect()
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

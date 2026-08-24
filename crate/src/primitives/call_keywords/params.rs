//! Resolves an in-module call to the parameters its def declares.

use super::*;

/// Maps each in-module call's callee offset to the parameters of the
/// top-level function it resolves to, over every function whose
/// decorators do not bind by position and whose name binds uniquely to
/// that one definition. Offsets come from `BindingAnalysis`, so a
/// shadowing local or aliased reference resolves elsewhere.
pub(crate) fn module_call_params(source: &Source) -> CallTargets<'_> {
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

/// Resolves `call`'s callee to the parameters it binds via `targets`, the
/// offset map [`module_call_params`] returns. `None` for an attribute
/// call, an unresolved name, or a callee outside the map.
pub(crate) fn resolve_call_params<'src>(
    call: &ExprCall,
    targets: &CallTargets<'src>,
) -> Option<&'src Parameters> {
    targets
        .get(&call.func.as_name_expr()?.range().start())
        .copied()
}

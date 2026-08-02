//! The gate deciding whether `from __future__ import annotations` is
//! inert. It clears when the target version defers annotation
//! evaluation per PEP 749, or when every name every annotation reads is
//! module-scope-defined before that annotation's offset, which a module
//! carrying no annotation at all satisfies with nothing to check.

use ruff_python_ast::{Expr, PythonVersion, helpers::any_over_expr};

use crate::{
    primitives::{binding::BindingAnalysis, walk::for_each_annotation},
    source::Source,
};

/// True when removing the `annotations` directive leaves every
/// annotation in `source` evaluating as it did.
pub(super) fn annotations_are_inert(source: &Source, target: Option<PythonVersion>) -> bool {
    target.is_some_and(PythonVersion::defers_annotations) || all_annotations_resolve_eagerly(source)
}

fn all_annotations_resolve_eagerly(source: &Source) -> bool {
    let analysis = source.binding_analysis();
    let mut resolved = true;
    for_each_annotation(&source.ast().body, |annotation| {
        resolved &= !annotation_is_unresolved(annotation, analysis);
    });
    resolved
}

/// True when `annotation` references a name no module-scope write
/// defines before it.
fn annotation_is_unresolved(annotation: &Expr, analysis: &BindingAnalysis) -> bool {
    any_over_expr(annotation, &|expr: &Expr| {
        expr.as_name_expr().is_some_and(|name| {
            name.ctx.is_load() && !analysis.is_defined_before(name.id.as_str(), name.range.start())
        })
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::no_annotation_anywhere("value = 1\n", None, true)]
    #[case::builtin_annotation_unresolved("def f(x: int) -> int:\n    return x\n", None, false)]
    #[case::py313_keeps_the_directive(
        "def f(x: int) -> int:\n    return x\n",
        Some(PythonVersion::PY313),
        false
    )]
    #[case::py314_defers_evaluation(
        "def f(x: int) -> int:\n    return x\n",
        Some(PythonVersion::PY314),
        true
    )]
    #[case::module_scope_name_resolves(
        "class Node:\n    pass\n\n\ndef visit(node: Node) -> Node:\n    return node\n",
        None,
        true
    )]
    fn annotations_are_inert_reads_each_branch(
        #[case] src: &str,
        #[case] target: Option<PythonVersion>,
        #[case] expected: bool,
    ) {
        assert_eq!(annotations_are_inert(&parse(src), target), expected);
    }
}

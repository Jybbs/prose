//! The gate deciding whether `from __future__ import annotations` is
//! inert. It clears when the target version defers annotation
//! evaluation per PEP 749, or when every name every annotation reads is
//! module-scope-defined before that annotation's offset, which a module
//! carrying no annotation at all satisfies with nothing to check. Where
//! `alphabetize-siblings` sorts definitions in the same pipeline, a name
//! a module-level class or function binds reads as unresolved whatever
//! its offset, the sort reseating that binding behind this rule.

use ruff_python_ast::{Expr, PythonVersion, helpers::any_over_expr};

use crate::{
    primitives::{
        binding::{BindingAnalysis, BindingKind},
        walk::for_each_annotation,
    },
    source::Source,
};

/// True when removing the `annotations` directive leaves every
/// annotation in `source` evaluating as it did, a definition's binding
/// reading as unresolved while `sorts_definitions`.
pub(super) fn annotations_are_inert(
    source: &Source,
    target: Option<PythonVersion>,
    sorts_definitions: bool,
) -> bool {
    target.is_some_and(PythonVersion::defers_annotations)
        || all_annotations_resolve_eagerly(source, sorts_definitions)
}

fn all_annotations_resolve_eagerly(source: &Source, sorts_definitions: bool) -> bool {
    let analysis = source.binding_analysis();
    let mut resolved = true;
    for_each_annotation(&source.ast().body, |annotation| {
        resolved &= !annotation_is_unresolved(annotation, analysis, sorts_definitions);
    });
    resolved
}

/// True when `annotation` references a name no module-scope write
/// defines before it, or one a module-level definition binds while
/// `sorts_definitions`.
fn annotation_is_unresolved(
    annotation: &Expr,
    analysis: &BindingAnalysis,
    sorts_definitions: bool,
) -> bool {
    any_over_expr(annotation, &|expr: &Expr| {
        expr.as_name_expr().is_some_and(|name| {
            name.ctx.is_load()
                && (!analysis.is_defined_before(name.id.as_str(), name.range.start())
                    || sorts_definitions && binds_a_definition(analysis, name.id.as_str()))
        })
    })
}

/// True when a module-level `def` or `class` writes `name`.
fn binds_a_definition(analysis: &BindingAnalysis, name: &str) -> bool {
    analysis
        .module_binding_kinds(name)
        .iter()
        .any(|kind| matches!(kind, BindingKind::ClassDef | BindingKind::FunctionDef))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    const CLASS_ABOVE_ITS_READER: &str =
        "class Node:\n    pass\n\n\ndef visit(node: Node) -> Node:\n    return node\n";

    #[rstest]
    #[case::no_annotation_anywhere("value = 1\n", None, false, true)]
    #[case::builtin_annotation_unresolved(
        "def f(x: int) -> int:\n    return x\n",
        None,
        false,
        false
    )]
    #[case::py313_keeps_the_directive(
        "def f(x: int) -> int:\n    return x\n",
        Some(PythonVersion::PY313),
        false,
        false
    )]
    #[case::py314_defers_evaluation(
        "def f(x: int) -> int:\n    return x\n",
        Some(PythonVersion::PY314),
        false,
        true
    )]
    #[case::module_scope_name_resolves(CLASS_ABOVE_ITS_READER, None, false, true)]
    #[case::sorted_definition_reads_unresolved(CLASS_ABOVE_ITS_READER, None, true, false)]
    #[case::import_resolves_under_the_sort(
        "from tree import Node\n\n\ndef visit(node: Node) -> Node:\n    return node\n",
        None,
        true,
        true
    )]
    #[case::py314_defers_evaluation_under_the_sort(
        CLASS_ABOVE_ITS_READER,
        Some(PythonVersion::PY314),
        true,
        true
    )]
    fn annotations_are_inert_reads_each_branch(
        #[case] src: &str,
        #[case] target: Option<PythonVersion>,
        #[case] sorts_definitions: bool,
        #[case] expected: bool,
    ) {
        assert_eq!(
            annotations_are_inert(&parse(src), target, sorts_definitions),
            expected
        );
    }
}

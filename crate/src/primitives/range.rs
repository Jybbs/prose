//! Parenthesis-aware source ranges for expression nodes.

use ruff_python_ast::{
    AnyNodeRef, Expr, ExprRef, StmtFunctionDef,
    token::{Tokens, parenthesized_range},
};
use ruff_text_size::{Ranged, TextRange};

/// Returns `expr`'s range widened to the explicit parentheses recovered
/// against `parent`, falling back to the bare expression range when none
/// enclose it.
pub(crate) fn paren_aware_range(expr: ExprRef, parent: AnyNodeRef, tokens: &Tokens) -> TextRange {
    parenthesized_range(expr, parent, tokens).unwrap_or_else(|| expr.range())
}

/// Returns the paren-aware range of `function`'s return annotation,
/// recovered against the function def.
pub(crate) fn return_annotation_range(
    annotation: &Expr,
    function: &StmtFunctionDef,
    tokens: &Tokens,
) -> TextRange {
    paren_aware_range(annotation.into(), function.into(), tokens)
}

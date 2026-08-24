//! Reads each grouping pair into the shed it earns, against the parse
//! the bare form produces, beside the calls a later explode reshapes.

use std::{borrow::Cow, cmp::Reverse};

use ruff_python_ast::{
    AnyNodeRef, Expr,
    token::{TokenKind, parenthesized_range},
};
use ruff_text_size::{Ranged, TextRange};

use super::flush::Flush;
use crate::{
    primitives::{
        inline::folded_line_form,
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener},
        walk::filter_map_over_exprs,
    },
    source::Source,
};

/// One grouping pair whose removal leaves the parse unchanged, carrying
/// its interior range, that interior's single-line form, `None` where
/// only a bracket inside the interior holds its breaks, and the sides on
/// which the pair runs into an identifier character.
pub(super) struct Candidate<'src> {
    pub(super) bare: Option<Cow<'src, str>>,
    pub(super) flush: Flush,
    pub(super) inner: TextRange,
    pub(super) pair: TextRange,
}

/// The candidate `expr` contributes, or `None` where no pair encloses
/// it, the pair carries syntax, or stripping the pair shifts the parse.
/// An interior no fold joins still qualifies where the brackets inside
/// it hold its breaks.
pub(super) fn candidate<'src>(
    source: &'src Source,
    expr: &'src Expr,
    parent: AnyNodeRef,
) -> Option<Candidate<'src>> {
    let pair = parenthesized_range(expr.into(), parent, source.tokens())?;
    // A walrus binding keeps its pair whatever the context, since the
    // grammar needs it almost everywhere, and a multi-line return
    // annotation belongs to `reflow-signatures`, so neither sheds here.
    if expr.is_named_expr()
        || (is_return_annotation(expr, parent) && source.contains_line_break(pair))
        || source.intersects_comment(pair)
    {
        return None;
    }
    let inner = expr.range();
    let bare = folded_line_form(expr, source.slice(inner));
    if bare.is_none() && !breaks_held_inside(source, inner) {
        return None;
    }
    let flush = Flush::of(source, pair);
    let probe = flush.padded(bare.as_deref().unwrap_or_else(|| source.slice(inner)));
    splice_preserves_tree(source, pair, &probe).then_some(Candidate {
        bare,
        flush,
        inner,
        pair,
    })
}

/// The range of every call no other call encloses, ascending by start,
/// the calls `reflow-calls` explodes first on an overflowing row.
pub(super) fn outermost_calls(source: &Source) -> Vec<TextRange> {
    let mut calls: Vec<TextRange> = filter_map_over_exprs(&source.ast().body, |expr| {
        expr.as_call_expr().map(Ranged::range)
    });
    calls.sort_unstable_by_key(|call| (call.start(), Reverse(call.end())));
    calls.dedup_by(|call, outer| outer.contains_range(*call));
    calls
}

/// True when every line break inside `inner` sits inside a bracket
/// `inner` itself opens, so the pair around it holds none of them.
fn breaks_held_inside(source: &Source, inner: TextRange) -> bool {
    let mut depth = 0_usize;
    source
        .tokens_overlapping(inner)
        .filter(|token| inner.contains(token.start()))
        .all(|token| {
            if is_opener(token.kind()) {
                depth += 1;
            } else if is_closer(token.kind()) {
                depth -= 1;
            }
            depth > 0 || token.kind() != TokenKind::NonLogicalNewline
        })
}

/// True when `expr` is the return annotation of the function `parent`.
fn is_return_annotation(expr: &Expr, parent: AnyNodeRef) -> bool {
    matches!(
        parent,
        AnyNodeRef::StmtFunctionDef(fd)
            if fd.returns.as_deref().is_some_and(|ann| ann.range() == expr.range())
    )
}

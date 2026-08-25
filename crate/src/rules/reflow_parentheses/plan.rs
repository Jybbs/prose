//! Reads each grouping pair into the reflow it earns, the shed against
//! the parse the bare form produces and the break against the rows its
//! interior divides into, beside the calls a later explode reshapes.

use std::{borrow::Cow, cmp::Reverse};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Expr,
    token::{TokenKind, parenthesized_range},
};
use ruff_text_size::{Ranged, TextRange};

use super::{
    chain::{Sheds, is_operator_chain},
    flush::Flush,
};
use crate::{
    primitives::{
        fracture::outermost,
        inline::folded_line_form,
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener},
        walk::{Descent, ParentedProbe, filter_map_over_exprs, walk_parented_exprs},
    },
    source::Source,
};

/// One grouping pair a reflow reaches, carrying the expression it
/// wraps, that expression's range and single-line form, whether the
/// pair links a wider chain as one of its operands, whether removing
/// the pair leaves the parse unchanged, and the sides on which the pair
/// runs into an identifier character.
pub(super) struct Candidate<'src> {
    pub(super) bare: Option<Cow<'src, str>>,
    pub(super) expr: &'src Expr,
    pub(super) flush: Flush,
    pub(super) inner: TextRange,
    pub(super) links: bool,
    pub(super) pair: TextRange,
    pub(super) sheds: bool,
}

impl Candidate<'_> {
    /// The two edits taking the pair's own parentheses out, each
    /// leaving a space where the side runs into an identifier
    /// character.
    pub(super) fn paren_removals(&self) -> [Edit; 2] {
        self.flush.flanking(
            TextRange::new(self.pair.start(), self.inner.start()),
            TextRange::new(self.inner.end(), self.pair.end()),
        )
    }
}

/// Collects a candidate per grouping pair, reading each expression
/// against the chain of nodes enclosing it.
struct Probe<'src> {
    found: Vec<Candidate<'src>>,
    source: &'src Source,
}

impl<'src> ParentedProbe<'src> for Probe<'src> {
    fn probe(
        &mut self,
        expr: &'src Expr,
        parent: AnyNodeRef<'src>,
        ancestors: &[AnyNodeRef<'src>],
    ) -> Descent {
        if let Some(found) = candidate(self.source, expr, parent, ancestors) {
            self.found.push(found);
        }
        Descent::Into
    }
}

/// True when every line break inside `inner` sits within a bracket
/// `inner` itself opens and `shed` reports this pass leaves standing,
/// so the pair around it holds none of them and the bracket that does
/// lays out the rows. A bracket `shed` reports coming out holds
/// nothing, leaving the reading the same before and after those pairs
/// shed.
pub(super) fn breaks_held_inside(source: &Source, inner: TextRange, shed: Sheds) -> bool {
    let mut depth = 0_usize;
    source
        .tokens_overlapping(inner)
        .filter(|token| inner.contains(token.start()))
        .all(|token| {
            if !shed(token.range()) {
                if is_opener(token.kind()) {
                    depth += 1;
                } else if is_closer(token.kind()) {
                    depth = depth.saturating_sub(1);
                }
            }
            depth > 0 || token.kind() != TokenKind::NonLogicalNewline
        })
}

/// Every grouping pair the module carries a candidate for, ascending by
/// start with an enclosing pair ahead of the pairs it holds.
pub(super) fn candidates(source: &Source) -> Vec<Candidate<'_>> {
    let mut probe = Probe {
        found: Vec::new(),
        source,
    };
    walk_parented_exprs(source.ast(), &mut probe);
    let mut found = probe.found;
    found.sort_unstable_by_key(|c| (c.pair.start(), Reverse(c.pair.end())));
    found
}

/// The range of every call no other call encloses, ascending by start,
/// the calls `reflow-calls` explodes first on an overflowing row.
pub(super) fn outermost_calls(source: &Source) -> Vec<TextRange> {
    outermost(filter_map_over_exprs(&source.ast().body, |expr| {
        expr.as_call_expr().map(Ranged::range)
    }))
}

/// The candidate `expr` contributes, or `None` where no pair encloses
/// it, the pair carries syntax, or neither direction is open to it. An
/// interior no fold joins still qualifies where the brackets inside it
/// hold its breaks and where a break can divide it into rows, with
/// `sheds` carrying whether removal itself holds the parse.
fn candidate<'src>(
    source: &'src Source,
    expr: &'src Expr,
    parent: AnyNodeRef,
    ancestors: &[AnyNodeRef],
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
    let bare = folded_line_form(source, expr, source.slice(inner));
    let chains = is_operator_chain(expr.into());
    if bare.is_none() && !chains && !breaks_held_inside(source, inner, &|_| false) {
        return None;
    }
    let flush = Flush::of(source, pair);
    let probe = flush.padded(bare.as_deref().unwrap_or_else(|| source.slice(inner)));
    let sheds = splice_preserves_tree(source, pair, &probe);
    (sheds || chains).then_some(Candidate {
        bare,
        expr,
        flush,
        inner,
        links: ancestors
            .iter()
            .filter_map(|node| node.as_expr_ref())
            .any(is_operator_chain),
        pair,
        sheds,
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

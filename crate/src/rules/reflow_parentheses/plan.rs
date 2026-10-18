//! Reads each grouping pair into the reflow it earns, the shed against
//! the parse the bare form produces and the break against the rows its
//! interior divides into, beside the calls a later explode reshapes. A
//! boolean chain or a conditional expression forming a test, a `return`
//! value, an assignment value, or the last element of a bare tuple
//! standing as one earns a pair the break adds around it.

use std::{borrow::Cow, cmp::Reverse};

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, token::TokenKind, visitor::source_order::TraversalSignal};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    chain::{Sheds, divides},
    flush::Flush,
    measure::fold_edits,
};
use crate::{
    primitives::{
        edit::apply_inline_edits,
        fracture::outermost,
        inline::{carries_a_continuation, folded_line_form},
        slots::starting_within,
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener, tokens_within},
        walk::{Interpolations, ParentedProbe, filter_map_over_exprs, walk_parented_exprs},
    },
    source::Source,
};

/// One grouping pair a reflow reaches, carrying the expression it
/// wraps, that expression's range and single-line form, whether the
/// pair links a wider chain as one of its operands, whether the pair
/// comes off without moving either the parse or the grouping a reader
/// sees, the sides on which the pair runs into an identifier character,
/// and whether a break lands every operand on a row of its own. A pair
/// the break adds spans the bare expression, so its `pair` and `inner`
/// match.
pub(super) struct Candidate<'src> {
    pub(super) bare: Option<Cow<'src, str>>,
    pub(super) expr: &'src Expr,
    pub(super) flush: Flush,
    pub(super) inner: TextRange,
    pub(super) links: bool,
    pub(super) pair: TextRange,
    pub(super) sheds: bool,
    pub(super) stacks: bool,
}

impl Candidate<'_> {
    /// True for a pair the break adds around a bare expression rather
    /// than one the source wrote.
    pub(super) fn adds(&self) -> bool {
        self.pair == self.inner
    }

    /// The two edits taking the pair's parenthesis characters out and
    /// nothing else, the removal a pair inside an open fold earns.
    pub(super) fn lone_paren_removals(&self) -> [Edit; 2] {
        let paren = TextSize::of('(');
        self.flush.flanking(
            TextRange::at(self.pair.start(), paren),
            TextRange::at(self.pair.end() - paren, paren),
        )
    }

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
    ) -> TraversalSignal {
        if let Some(found) = candidate(self.source, expr, parent, ancestors) {
            self.found.push(found);
        }
        TraversalSignal::Traverse
    }
}

/// True when every line break inside `inner` sits within a bracket
/// `inner` itself opens and `shed` reports this pass leaves standing.
pub(super) fn breaks_held_inside(source: &Source, inner: TextRange, shed: Sheds) -> bool {
    let mut depth = 0_usize;
    tokens_within(source, inner).all(|token| {
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
    let written: Vec<TextSize> = found
        .iter()
        .filter(|c| !c.adds())
        .map(|c| c.pair.start())
        .collect();
    found.retain(|c| !c.adds() || folds_inside_pairs(source, c.inner, &written));
    found
}

/// True where every line break inside `inner` closes by folding a pair the
/// source wrote, each break sitting directly inside a pair whose `(` opens
/// at one of `written`, and no comment, no string spanning rows, and no
/// method-chain link opening a row sits inside `inner`.
fn folds_inside_pairs(source: &Source, inner: TextRange, written: &[TextSize]) -> bool {
    let mut open = Vec::new();
    let mut after_break = false;
    tokens_within(source, inner).all(|token| {
        let kind = token.kind();
        if is_opener(kind) {
            open.push(token.start());
        } else if is_closer(kind) {
            open.pop();
        }
        let holds = match kind {
            TokenKind::NonLogicalNewline => open
                .last()
                .is_some_and(|start| written.binary_search(start).is_ok()),
            TokenKind::Comment => false,
            TokenKind::Dot => !after_break,
            _ => !source.contains_line_break(token.range()),
        };
        after_break = kind == TokenKind::NonLogicalNewline;
        holds
    })
}

/// The range of every call no other call encloses, ascending by start,
/// the calls `reflow-calls` explodes first on an overflowing row.
pub(super) fn outermost_calls(source: &Source) -> Vec<TextRange> {
    outermost(filter_map_over_exprs(
        &source.ast().body,
        Interpolations::Skip,
        |expr| expr.as_call_expr().map(Ranged::range),
    ))
}

/// The columns the pairs shed inside `range` give back, two per pair
/// less the spaces its flush sides keep.
pub(super) fn shed_columns(range: TextRange, candidates: &[Candidate]) -> usize {
    shedding_inside(range, candidates)
        .map(|other| 2 - other.flush.spaces())
        .sum()
}

/// The candidates this pass sheds whose pair sits inside `range`.
pub(super) fn shedding_inside<'c, 'src>(
    range: TextRange,
    candidates: &'c [Candidate<'src>],
) -> impl Iterator<Item = &'c Candidate<'src>> {
    starting_within(candidates, range, |other| other.pair.start())
        .filter(move |other| other.sheds && range.contains_range(other.pair))
}

/// True where `expr` joins at one boolean operator and the node
/// holding it joins at the other.
fn binds_apart(expr: &Expr, parent: AnyNodeRef) -> bool {
    let Expr::BoolOp(inner) = expr else {
        return false;
    };
    matches!(parent, AnyNodeRef::ExprBoolOp(outer) if outer.op != inner.op)
}

/// The candidate `expr` contributes, or `None` where the pair carries
/// syntax or neither direction is open to it. An interior no fold joins
/// still qualifies where the brackets inside it hold its breaks and
/// where a break can divide it into rows, with `sheds` carrying whether
/// removal holds both the parse and the grouping a reader sees. An
/// expression no pair encloses contributes the pair [`added`] reads.
fn candidate<'src>(
    source: &'src Source,
    expr: &'src Expr,
    parent: AnyNodeRef,
    ancestors: &[AnyNodeRef],
) -> Option<Candidate<'src>> {
    let Some(pair) = source.parenthesized_range(expr.into(), parent) else {
        return added(source, expr, parent, ancestors);
    };
    // A walrus binding keeps its pair whatever the context, a pair
    // holding a comment stays, a multi-line return annotation belongs
    // to `reflow-signatures`, and a backslash continuing a row inside
    // the pair belongs to `shed-backslash-continuations`, so none of
    // them yields a candidate.
    if expr.is_named_expr()
        || (is_return_annotation(expr, parent) && source.contains_line_break(pair))
        || !source.comment_ranges().comments_in_range(pair).is_empty()
        || carries_a_continuation(source.slice(pair))
    {
        return None;
    }
    let inner = expr.range();
    let bare = folded_line_form(source, expr, source.slice(inner));
    // A conditional expression its author spread across rows has no fold
    // to rejoin it, so only a one-row conditional divides.
    let chains = divides(expr.into()) && (bare.is_some() || !expr.is_if_expr());
    if bare.is_none() && !chains && !breaks_held_inside(source, inner, &|_| false) {
        return None;
    }
    let flush = Flush::of(source, pair);
    let probe = flush.padded(bare.as_deref().unwrap_or_else(|| source.slice(inner)));
    let sheds = splice_preserves_tree(source, pair, &probe) && !binds_apart(expr, parent);
    (sheds || chains).then_some(Candidate {
        bare,
        expr,
        flush,
        inner,
        links: ancestors
            .iter()
            .filter_map(|node| node.as_expr_ref())
            .any(divides),
        pair,
        sheds,
        stacks: stacks(expr, parent),
    })
}

/// The candidate for a pair the break adds around `expr`, a boolean
/// chain or a conditional expression that forms the test of an `if`,
/// `elif`, or `while`, the value of a `return` or an assignment, or the
/// last element of a bare tuple standing as such a value, read at the
/// one row folding its soft breaks leaves. `None` for any other
/// expression, since a pair anywhere else would sit inside a bracket or
/// open mid-row. [`candidates`] keeps one spanning rows only where
/// [`folds_inside_pairs`] holds.
fn added<'src>(
    source: &'src Source,
    expr: &'src Expr,
    parent: AnyNodeRef,
    ancestors: &[AnyNodeRef],
) -> Option<Candidate<'src>> {
    let inner = expr.range();
    let slot = match parent {
        AnyNodeRef::ExprTuple(tuple) => {
            !tuple.parenthesized
                && tuple.elts.last().is_some_and(|last| last.range() == inner)
                && matches!(ancestors, [.., held, _] if stands_as_value(tuple.range(), *held))
        }
        _ => is_test(inner, parent) || stands_as_value(inner, parent),
    };
    if !slot || !matches!(expr, Expr::BoolOp(_) | Expr::If(_)) {
        return None;
    }
    Some(Candidate {
        bare: Some(apply_inline_edits(
            source,
            inner,
            &fold_edits(source, inner),
        )),
        expr,
        flush: Flush::of(source, inner),
        inner,
        links: false,
        pair: inner,
        sheds: false,
        stacks: stacks(expr, parent),
    })
}

/// True where `range` is the test of the `if`, `elif`, or `while`
/// statement `parent`.
fn is_test(range: TextRange, parent: AnyNodeRef) -> bool {
    match parent {
        AnyNodeRef::StmtIf(stmt) => {
            stmt.test.range() == range
                || stmt.elif_else_clauses.iter().any(|clause| {
                    clause
                        .test
                        .as_ref()
                        .is_some_and(|test| test.range() == range)
                })
        }
        AnyNodeRef::StmtWhile(stmt) => stmt.test.range() == range,
        _ => false,
    }
}

/// True where `expr` is a boolean chain forming the test of `parent`,
/// the shape a break lands one operand per row whatever its width.
fn stacks(expr: &Expr, parent: AnyNodeRef) -> bool {
    expr.is_bool_op_expr() && is_test(expr.range(), parent)
}

/// True where `range` is the value the `return` or assignment `node`
/// holds.
fn stands_as_value(range: TextRange, node: AnyNodeRef) -> bool {
    let value = match node {
        AnyNodeRef::StmtAnnAssign(stmt) => stmt.value.as_deref(),
        AnyNodeRef::StmtAssign(stmt) => Some(&*stmt.value),
        AnyNodeRef::StmtAugAssign(stmt) => Some(&*stmt.value),
        AnyNodeRef::StmtReturn(stmt) => stmt.value.as_deref(),
        _ => None,
    };
    value.is_some_and(|value| value.range() == range)
}

/// True when `expr` is the return annotation of the function `parent`.
fn is_return_annotation(expr: &Expr, parent: AnyNodeRef) -> bool {
    matches!(
        parent,
        AnyNodeRef::StmtFunctionDef(fd)
            if fd.returns.as_deref().is_some_and(|ann| ann.range() == expr.range())
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// The text of each expression in `src` the break adds a pair around.
    fn added_around(src: &str) -> Vec<String> {
        let source = parse(src);
        candidates(&source)
            .iter()
            .filter(|candidate| candidate.adds())
            .map(|candidate| source.slice(candidate.inner).to_owned())
            .collect()
    }

    #[rstest]
    #[case("if a and b:\n    pass\n", "a and b")]
    #[case("if x:\n    pass\nelif a or b:\n    pass\n", "a or b")]
    #[case("while a and b:\n    pass\n", "a and b")]
    #[case("def f():\n    return a if b else c\n", "a if b else c")]
    #[case("x = a and b\n", "a and b")]
    #[case("x: int = a if b else c\n", "a if b else c")]
    #[case("x += a or b\n", "a or b")]
    #[case("x = y, a if b else c\n", "a if b else c")]
    #[case("x = a if (b\n    and c) else d\n", "a if (b\n    and c) else d")]
    fn added_pairs_open_on_a_statement_slot(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(added_around(src), [expected]);
    }

    #[rstest]
    #[case("assert a and b\n")]
    #[case("x = a if b else c, y\n")]
    #[case("x = (y, a if b else c)\n")]
    #[case("x = f(a and b)\n")]
    #[case("x = a + b\n")]
    #[case("x = (a and b)\n")]
    #[case("x = a and f(\n    b\n)\n")]
    #[case("x = a if [b,\n    c] else d\n")]
    #[case("x = a if (b.c\n    .d) else e\n")]
    #[case("x = a if (b  # note\n    and c) else d\n")]
    fn added_pairs_skip_every_other_expression(#[case] src: &str) {
        assert!(added_around(src).is_empty());
    }

    #[rstest]
    #[case("if a and b:\n    pass\n", true)]
    #[case("while (a or b):\n    pass\n", true)]
    #[case("if a if b else c:\n    pass\n", false)]
    #[case("x = a and b\n", false)]
    fn a_boolean_test_stacks_its_operands(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        assert_eq!(candidates(&source)[0].stacks, expected);
    }
}

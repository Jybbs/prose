//! Item classification for collection layout: which expressions are
//! atomic, layoutable, or force expansion, and how an atomic run
//! partitions into flow and one-per-line segments.

use std::ops::Range;

use ruff_python_ast::{Expr, helpers::is_dotted_name};

use crate::primitives::layout::is_layoutable;

/// Describes how a contiguous slice of items should lay out.
#[derive(Debug, PartialEq)]
pub(super) enum Segment {
    /// Items in the range flow across as few balanced lines as fit.
    Flow(Range<usize>),
    /// Each item in the range goes on its own line.
    OnePerLine(Range<usize>),
}

/// Returns `true` when `gap` is zero or more ASCII spaces, then
/// `:`, then one ASCII space.
pub(super) fn is_align_colons_gap(gap: &str) -> bool {
    gap.strip_suffix(": ")
        .is_some_and(|prefix| prefix.bytes().all(|b| b == b' '))
}

/// True for expressions that render as a single compact token and
/// therefore do not benefit from a dedicated line. Covers literals,
/// dotted names, and unary operations over atomic operands. Starred
/// expressions are non-atomic so a spread splits surrounding atomics
/// into independent runs.
pub(super) fn is_atomic(expr: &Expr) -> bool {
    std::iter::successors(Some(expr), |e| {
        e.as_unary_op_expr().map(|u| u.operand.as_ref())
    })
    .any(|e| e.is_literal_expr() || is_dotted_name(e))
}

/// True for the collapse-only forms, a subscript whose `[index]` joins
/// onto one line whatever the index shape and the four comprehensions,
/// each joining when it fits and never expanding the way a literal does.
pub(super) fn is_collapse_only(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::DictComp(_)
            | Expr::Generator(_)
            | Expr::ListComp(_)
            | Expr::SetComp(_)
            | Expr::Subscript(_)
    )
}

/// True for the bracketed expressions the visitor measures for a
/// single-line collapse: the four collection literals plus the
/// collapse-only forms, a subscript and the four comprehensions.
pub(super) fn is_collapsible(expr: &Expr) -> bool {
    is_layoutable(expr) || is_collapse_only(expr)
}

/// True for a `Dict`, `List`, or `Set` shape the expand path
/// canonicalizes. Multi-item `List` and `Set` qualify. Any
/// non-empty `Dict` qualifies. Tuples and empty collections
/// collapse only, never expand.
pub(super) fn requires_expand(expr: &Expr) -> bool {
    match expr {
        Expr::Dict(d) => !d.is_empty(),
        Expr::List(l) => l.len() > 1,
        Expr::Set(s) => s.len() > 1,
        _ => false,
    }
}

/// Partitions `atomics` into segments. Every contiguous run of
/// atomic items becomes one `Flow` segment. Every non-atomic item
/// becomes a singleton `OnePerLine` segment. Non-atomic items always
/// break atomic runs.
pub(super) fn segments(atomics: &[bool]) -> Vec<Segment> {
    atomics
        .chunk_by(|a, b| a == b)
        .scan(0, |start, chunk| {
            let range = *start..*start + chunk.len();
            *start += chunk.len();
            Some(if chunk[0] {
                Segment::Flow(range)
            } else {
                Segment::OnePerLine(range)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn align_colons_gap_accepts_canonical_and_padded_forms() {
        assert!(is_align_colons_gap(": "));
        assert!(is_align_colons_gap(" : "));
        assert!(is_align_colons_gap("    : "));
    }

    #[test]
    fn align_colons_gap_rejects_non_padding_shapes() {
        assert!(!is_align_colons_gap(":"));
        assert!(!is_align_colons_gap(":  "));
        assert!(!is_align_colons_gap(" :"));
        assert!(!is_align_colons_gap("\t: "));
        assert!(!is_align_colons_gap(""));
    }

    #[rstest]
    #[case("[a]", true)]
    #[case("{b}", true)]
    #[case("(c, d)", true)]
    #[case("{e: f}", true)]
    #[case("g[h]", true)]
    #[case("[x for x in y]", true)]
    #[case("{x for x in y}", true)]
    #[case("{k: v for k, v in y}", true)]
    #[case("(x for x in y)", true)]
    #[case("plain", false)]
    #[case("a + b", false)]
    fn is_collapsible_covers_literals_subscripts_and_comprehensions(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let stmt = &source.ast().body[0];
        let expr = &stmt.as_expr_stmt().expect("expression statement").value;
        assert_eq!(is_collapsible(expr), expected);
    }

    #[test]
    fn segments_partitions_alternating_atomic_runs() {
        let result = segments(&[true, true, false, true, false, false]);
        assert_eq!(
            result,
            vec![
                Segment::Flow(0..2),
                Segment::OnePerLine(2..3),
                Segment::Flow(3..4),
                Segment::OnePerLine(4..6),
            ],
        );
    }

    #[test]
    fn segments_returns_empty_for_empty_input() {
        assert!(segments(&[]).is_empty());
    }
}

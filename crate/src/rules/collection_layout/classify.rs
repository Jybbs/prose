//! Item classification for collection layout: which expressions are
//! atomic, layoutable, or force expansion, and how an atomic run
//! partitions into flow and one-per-line segments, plus whether a
//! literal's source text is already the flush bracketed column the
//! expand path emits.

use std::ops::Range;

use ruff_python_ast::{Expr, helpers::is_dotted_name};

use crate::primitives::{
    layout::{flush_bracket_open, is_layoutable},
    orderer::slot_runs,
};

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
    split_colon_gap(gap).is_some_and(|(_, tail)| tail == " ")
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

/// True when `slice`, a collection literal's source text, already
/// carries the flush bracket shape the expand path emits, its opening
/// bracket ending its line and its closing bracket opening its own.
pub(super) fn is_column_shaped(slice: &str) -> bool {
    flush_bracket_open(slice).is_some_and(|(_, body)| {
        body.rsplit_once('\n')
            .is_some_and(|(_, close)| close.trim_start().len() == 1)
    })
}

/// The ASCII-space run `gap` opens with when those spaces sit directly
/// before its `:`, the padding `align_colons` holds a dict key at.
/// Returns `""` for a canonical `": "` and for any other gap shape.
pub(super) fn pre_colon_padding(gap: &str) -> &str {
    split_colon_gap(gap).map_or("", |(padding, _)| padding)
}

/// True for a `Dict`, `List`, `Set`, or parenthesized `Tuple` shape
/// the expand path canonicalizes. Multi-item `List`, `Set`, and
/// parenthesized `Tuple` qualify, as does any non-empty `Dict`. A bare
/// tuple carries no bracket pair to hang broken lines on, and an empty
/// or single-item collection has nothing to flow.
pub(super) fn requires_expand(expr: &Expr) -> bool {
    match expr {
        Expr::Dict(d) => !d.is_empty(),
        Expr::List(l) => l.len() > 1,
        Expr::Set(s) => s.len() > 1,
        Expr::Tuple(t) => t.parenthesized && t.len() > 1,
        _ => false,
    }
}

/// Partitions `atomics` into segments. Each contiguous run of atomic
/// items becomes one `Flow` segment and each contiguous run of
/// non-atomic items one `OnePerLine` segment, so a non-atomic item
/// breaks the atomic run around it.
pub(super) fn segments(atomics: &[bool]) -> Vec<Segment> {
    slot_runs(atomics, |a, b| a == b)
        .map(|run| {
            if atomics[run.start] {
                Segment::Flow(run)
            } else {
                Segment::OnePerLine(run)
            }
        })
        .collect()
}

/// Splits `gap`, the span between a dict key and its value, at its
/// first `:` into the run before it and the tail after. Returns `None`
/// when `gap` carries no `:` or that leading run holds anything but
/// ASCII spaces.
fn split_colon_gap(gap: &str) -> Option<(&str, &str)> {
    let (padding, tail) = gap.split_once(':')?;
    padding
        .bytes()
        .all(|b| b == b' ')
        .then_some((padding, tail))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

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
        let expr = first_expr(&source);
        assert_eq!(is_collapsible(expr), expected);
    }

    #[rstest]
    #[case("[\n    1,\n    2,\n]", true)]
    #[case("{\n    'a': 1\n}", true)]
    #[case("(\n    'only',\n)", true)]
    #[case("[\r\n    1,\r\n]", true)]
    #[case("[1,\n 2]", false)]
    #[case("(\n    value,)", false)]
    #[case("{\n}", false)]
    #[case("[1, 2]", false)]
    fn is_column_shaped_wants_both_brackets_alone_on_their_lines(
        #[case] slice: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_column_shaped(slice), expected);
    }

    #[rstest]
    #[case(": ", "")]
    #[case(" : ", " ")]
    #[case("    : ", "    ")]
    #[case(" :\n        ", " ")]
    #[case(":\n        ", "")]
    #[case("\t: ", "")]
    #[case(" # note\n: ", "")]
    #[case("", "")]
    fn pre_colon_padding_keeps_only_a_leading_space_run(#[case] gap: &str, #[case] expected: &str) {
        assert_eq!(pre_colon_padding(gap), expected);
    }

    #[rstest]
    #[case("(a, b)", true)]
    #[case("(a,)", false)]
    #[case("()", false)]
    #[case("a, b, c", false)]
    #[case("(a + b)", false)]
    #[case("[a, b]", true)]
    fn requires_expand_gates_parenthesized_multi_item_tuples(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(requires_expand(expr), expected);
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

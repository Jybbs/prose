//! Item classification for collection layout: which expressions are
//! atomic, layoutable, or force expansion, and how an atomic run
//! partitions into flow and one-per-line segments.

use std::ops::Range;

use ruff_python_ast::{Expr, helpers::is_dotted_name};

use crate::primitives::slots::slot_runs;

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
/// dotted names, unary operations over atomic operands, and an
/// attribute reached off any of them, so `None.__new__` reads the way
/// `object.__new__` does. Starred expressions are non-atomic so a
/// spread takes its run to one item per line.
pub(super) fn is_atomic(expr: &Expr) -> bool {
    std::iter::successors(Some(expr), |e: &&Expr| match e {
        Expr::Attribute(a) => Some(a.value.as_ref()),
        Expr::UnaryOp(u) => Some(u.operand.as_ref()),
        _ => None,
    })
    .any(|e| e.is_literal_expr() || is_dotted_name(e))
}

/// The ASCII-space run `gap` opens with when those spaces sit directly
/// before its `:`, the padding `align_colons` holds a dict key at.
/// Returns `""` for a canonical `": "` and for any other gap shape.
pub(super) fn pre_colon_padding(gap: &str) -> &str {
    split_colon_gap(gap).map_or("", |(padding, _)| padding)
}

/// Partitions `atomics` into segments. Each contiguous run of atomic
/// items becomes one `Flow` segment and each contiguous run of
/// non-atomic items one `OnePerLine` segment, so a non-atomic item
/// breaks the atomic run around it.
///
/// A `reordered` run answers one segment for the whole of itself
/// instead, flowing only while every item is atomic. `alphabetize`
/// permutes a set's members, so a partition read off their arrival
/// order repartitions on the pass after the sort and never settles,
/// whereas a list or tuple keeps the order the author wrote.
pub(super) fn segments(atomics: &[bool], reordered: bool) -> Vec<Segment> {
    if reordered && !atomics.is_empty() {
        let run = 0..atomics.len();
        return if atomics.iter().all(|atomic| *atomic) {
            vec![Segment::Flow(run)]
        } else {
            vec![Segment::OnePerLine(run)]
        };
    }
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

    #[test]
    fn segments_partitions_alternating_atomic_runs() {
        let result = segments(&[true, true, false, true, false, false], false);
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

    #[rstest]
    #[case::every_item_atomic(&[true, true], Segment::Flow(0..2))]
    #[case::any_item_non_atomic(&[true, false, true], Segment::OnePerLine(0..3))]
    fn segments_answers_one_segment_for_a_reordered_run(
        #[case] atomics: &[bool],
        #[case] expected: Segment,
    ) {
        assert_eq!(segments(atomics, true), vec![expected]);
    }

    #[test]
    fn segments_returns_empty_for_empty_input() {
        assert!(segments(&[], false).is_empty());
    }
}

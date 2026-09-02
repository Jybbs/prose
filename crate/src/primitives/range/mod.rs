//! Parenthesis-aware source ranges for expression nodes, the extent a
//! slice of ranged items covers, the merge of a span list into its
//! disjoint runs, and the spans a deletion takes out of a
//! comma-separated list.

use std::ops::Range;

use ruff_python_ast::{Expr, StmtFunctionDef};
use ruff_text_size::{Ranged, TextRange};

use crate::{primitives::slots::slot_runs, source::Source};

/// Total source extent covered by `blocks`. Requires non-empty input.
pub(crate) fn blocks_span<T: Ranged>(blocks: &[T]) -> TextRange {
    blocks
        .iter()
        .map(Ranged::range)
        .reduce(TextRange::cover)
        .expect("non-empty blocks")
}

/// The spans deleting every member of a comma-separated list that
/// `reject` names by index, one span per contiguous run of rejected
/// members, each carrying the separator binding it to the survivor
/// beside it. `members` holds every member's range in source order,
/// widened to any grouping parentheses around it, since a span that
/// stops inside those parentheses leaves a stray bracket behind. Empty
/// where every member survives and where none does, a list losing
/// everything going whole rather than member by member.
pub(crate) fn dropped_member_spans(
    members: &[TextRange],
    reject: impl Fn(usize) -> bool,
) -> Vec<TextRange> {
    let rejected: Vec<bool> = (0..members.len()).map(reject).collect();
    slot_runs(&rejected, |a, b| a == b)
        .filter(|run| rejected[run.start] && run.len() < members.len())
        .map(|run| member_deletion_span(members, run))
        .collect()
}

/// `spans` sorted, with every overlapping or touching pair folded into
/// the span covering both.
pub(crate) fn merged_spans(mut spans: Vec<TextRange>) -> Vec<TextRange> {
    spans.sort_unstable_by_key(Ranged::start);
    spans.dedup_by(|next, prev| {
        let meets = next.start() <= prev.end();
        if meets {
            *prev = prev.cover(*next);
        }
        meets
    });
    spans
}

/// True where `range` sits wholly inside one of `spans`, ascending
/// and disjoint.
pub(crate) fn covers(range: TextRange, spans: &[TextRange]) -> bool {
    let at = spans.partition_point(|span| span.end() < range.end());
    spans.get(at).is_some_and(|span| span.contains_range(range))
}

/// True where `range` overlaps one of `spans`, ascending and disjoint,
/// by at least one byte.
pub(crate) fn overlaps(range: TextRange, spans: &[TextRange]) -> bool {
    spans.binary_search_by(|span| span.ordering(range)).is_ok()
}

/// Returns the paren-aware range of `function`'s return annotation,
/// recovered against the function def.
pub(crate) fn return_annotation_range(
    annotation: &Expr,
    function: &StmtFunctionDef,
    source: &Source,
) -> TextRange {
    source.paren_aware_range(annotation.into(), function.into())
}

/// The span deleting `run` leaves in `members`, reaching forward to the
/// following member's start where one exists and back to the preceding
/// member's end otherwise. `run` never covers every member, the caller
/// having filtered that case out so one neighbor always survives.
fn member_deletion_span(members: &[TextRange], run: Range<usize>) -> TextRange {
    match (run.start.checked_sub(1), members.get(run.end)) {
        (_, Some(following)) => TextRange::new(members[run.start].start(), following.start()),
        (Some(preceding), None) => {
            TextRange::new(members[preceding].end(), members[run.end - 1].end())
        }
        (None, None) => unreachable!("invariant: a deleted member keeps at least one sibling"),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::range;

    #[rstest]
    #[case(&[0], vec![(0, 6)])]
    #[case(&[1], vec![(6, 12)])]
    #[case(&[0, 1], vec![(0, 12)])]
    #[case(&[0, 2], vec![(0, 6), (10, 16)])]
    #[case(&[], vec![])]
    #[case(&[0, 1, 2], vec![])]
    fn dropped_member_spans_covers_each_run_with_its_separator(
        #[case] rejected: &[usize],
        #[case] expected: Vec<(u32, u32)>,
    ) {
        let members = [range(0, 4), range(6, 10), range(12, 16)];
        let pairs: Vec<(u32, u32)> = dropped_member_spans(&members, |i| rejected.contains(&i))
            .iter()
            .map(|span| (span.start().into(), span.end().into()))
            .collect();
        assert_eq!(pairs, expected);
    }

    #[rstest]
    #[case(0..1, 0, 6)]
    #[case(1..2, 6, 12)]
    #[case(2..3, 10, 16)]
    #[case(0..2, 0, 12)]
    #[case(1..3, 4, 16)]
    fn member_deletion_span_takes_the_separator_on_the_surviving_side(
        #[case] run: Range<usize>,
        #[case] start: u32,
        #[case] end: u32,
    ) {
        let members = [range(0, 4), range(6, 10), range(12, 16)];
        assert_eq!(member_deletion_span(&members, run), range(start, end));
    }

    #[rstest]
    #[case::inside_one_span(range(4, 6), true)]
    #[case::flush_with_a_span(range(2, 8), true)]
    #[case::reaching_past_a_span(range(4, 9), false)]
    #[case::empty_inside_a_span(range(5, 5), true)]
    #[case::empty_at_a_span_end(range(8, 8), true)]
    #[case::across_two_spans(range(6, 12), false)]
    #[case::ahead_of_every_span(range(0, 1), false)]
    fn covers_reads_a_range_wholly_inside_one_span(
        #[case] span: TextRange,
        #[case] expected: bool,
    ) {
        assert_eq!(covers(span, &[range(2, 8), range(10, 14)]), expected);
    }

    #[rstest]
    #[case::inside_one_span(range(4, 6), true)]
    #[case::one_byte_into_a_span(range(7, 9), true)]
    #[case::touching_a_span_end(range(8, 9), false)]
    #[case::touching_a_span_start(range(1, 2), false)]
    #[case::empty_inside_a_span(range(5, 5), true)]
    #[case::spanning_two_spans(range(6, 12), true)]
    #[case::between_two_spans(range(9, 10), false)]
    fn overlaps_reads_a_range_meeting_a_span_by_a_byte(
        #[case] span: TextRange,
        #[case] expected: bool,
    ) {
        assert_eq!(overlaps(span, &[range(2, 8), range(10, 14)]), expected);
    }
}

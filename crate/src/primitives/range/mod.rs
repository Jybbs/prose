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
    blocks[0]
        .range()
        .cover(blocks.last().expect("non-empty blocks").range())
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
    #[case(0..2, 0, 12)]
    #[case(1..3, 4, 16)]
    fn member_deletion_span_steps_over_a_run(
        #[case] run: Range<usize>,
        #[case] start: u32,
        #[case] end: u32,
    ) {
        let members = [range(0, 4), range(6, 10), range(12, 16)];
        assert_eq!(member_deletion_span(&members, run), range(start, end));
    }

    #[rstest]
    #[case(0, 0, 6)]
    #[case(1, 6, 12)]
    #[case(2, 10, 16)]
    fn member_deletion_span_takes_the_separator_on_the_surviving_side(
        #[case] index: usize,
        #[case] start: u32,
        #[case] end: u32,
    ) {
        let members = [range(0, 4), range(6, 10), range(12, 16)];
        assert_eq!(
            member_deletion_span(&members, index..index + 1),
            range(start, end),
        );
    }
}

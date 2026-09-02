//! Slot arithmetic over a member list, the runs adjacent members form,
//! the inverse of a reordering, and the member an offset falls in.

use std::ops::Range;

use ruff_text_size::{Ranged, TextRange, TextSize};

/// The item of `items` whose start is at or before `offset`, `None`
/// ahead of the first item.
pub(crate) fn item_holding<T: Ranged>(items: &[T], offset: TextSize) -> Option<&T> {
    slot_holding(items, offset).map(|slot| &items[slot])
}

/// Slot ranges of each run of two or more adjacent items that each
/// satisfy `qualifies`, an item failing it bounding the runs on either
/// side.
pub(crate) fn runs_where<T>(
    items: &[T],
    mut qualifies: impl FnMut(&T) -> bool,
) -> Vec<Range<usize>> {
    slot_runs(items, |a, b| qualifies(a) && qualifies(b))
        .filter(|run| run.len() >= 2)
        .collect()
}

/// The slot of the `items` entry whose start is at or before `offset`,
/// `None` ahead of the first item.
pub(crate) fn slot_holding<T: Ranged>(items: &[T], offset: TextSize) -> Option<usize> {
    items
        .partition_point(|item| item.start() <= offset)
        .checked_sub(1)
}

/// Inverts `order` into the slot each item index occupies, the reverse
/// of the index-per-slot mapping `order` itself holds. Reading
/// `slot_positions(order)[idx]` answers where item `idx` landed.
pub(crate) fn slot_positions(order: &[usize]) -> Vec<usize> {
    let mut positions = vec![0usize; order.len()];
    for (slot, &idx) in order.iter().enumerate() {
        positions[idx] = slot;
    }
    positions
}

/// Slot ranges of each run of adjacent items whose pairwise neighbors
/// satisfy `adjacent`, singletons included. An empty `items` yields no
/// run.
pub(crate) fn slot_runs<T>(
    items: &[T],
    adjacent: impl FnMut(&T, &T) -> bool,
) -> impl Iterator<Item = Range<usize>> {
    let mut start = 0;
    items.chunk_by(adjacent).map(move |chunk| {
        let end = start + chunk.len();
        let run = start..end;
        start = end;
        run
    })
}

/// The items of `items`, ascending by `start`, whose start falls
/// inside `range`.
pub(crate) fn starting_within<T>(
    items: &[T],
    range: TextRange,
    start: impl Fn(&T) -> TextSize + Copy,
) -> impl Iterator<Item = &T> {
    let from = items.partition_point(|item| start(item) < range.start());
    items[from..]
        .iter()
        .take_while(move |item| start(item) < range.end())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{parse, range};

    /// A comment ahead of two statements, the first starting at 7 and
    /// the second at 14.
    const LEAD_COMMENT: &str = "# lead\nx = 1\n\ny = 2\n";

    #[rstest]
    #[case::ahead_of_the_first_item(0, None)]
    #[case::inside_an_item(10, Some(7))]
    #[case::past_the_last_item(20, Some(14))]
    fn item_holding_reads_the_item_its_slot_names(#[case] offset: u32, #[case] start: Option<u32>) {
        let source = parse(LEAD_COMMENT);
        assert_eq!(
            item_holding(&source.ast().body, TextSize::new(offset)).map(Ranged::start),
            start.map(TextSize::new)
        );
    }

    #[test]
    fn runs_where_bounds_runs_at_each_failing_item() {
        let items = [1, 1, 0, 1, 1, 1];
        assert_eq!(runs_where(&items, |&n| n == 1), vec![0..2, 3..6]);
    }

    #[rstest]
    #[case::ahead_of_the_first_statement(0, None)]
    #[case::statement_start(7, Some(0))]
    #[case::inside_a_statement(10, Some(0))]
    #[case::between_statements(13, Some(0))]
    #[case::past_the_last_statement(20, Some(1))]
    fn slot_holding_reads_the_statement_starting_at_or_before_the_offset(
        #[case] offset: u32,
        #[case] expected: Option<usize>,
    ) {
        let source = parse(LEAD_COMMENT);
        assert_eq!(
            slot_holding(&source.ast().body, TextSize::new(offset)),
            expected
        );
    }

    #[test]
    fn slot_positions_inverts_an_order() {
        assert_eq!(slot_positions(&[2, 0, 1]), vec![1, 2, 0]);
    }

    #[test]
    fn slot_runs_keeps_singleton_runs() {
        let items = [1, 1, 2, 3, 3, 3];
        assert_eq!(
            slot_runs(&items, |a, b| a == b).collect::<Vec<_>>(),
            vec![0..2, 2..3, 3..6]
        );
    }

    #[rstest]
    #[case::inside_the_range(range(4, 12), &[4, 8])]
    #[case::opening_at_the_range_start(range(8, 20), &[8, 12, 16])]
    #[case::closing_at_the_range_end(range(0, 8), &[0, 4])]
    #[case::empty_range(range(8, 8), &[])]
    #[case::past_every_item(range(21, 30), &[])]
    fn starting_within_takes_the_items_opening_inside_the_range(
        #[case] span: TextRange,
        #[case] expected: &[u32],
    ) {
        let items: Vec<TextRange> = (0..5).map(|i| range(i * 4, i * 4 + 3)).collect();
        let starts: Vec<u32> = starting_within(&items, span, Ranged::start)
            .map(|item| item.start().to_u32())
            .collect();

        assert_eq!(starts, expected);
    }
}

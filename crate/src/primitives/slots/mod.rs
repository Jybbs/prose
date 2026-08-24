//! Slot arithmetic over a member list, the runs adjacent members form
//! and the inverse of a reordering.

use std::ops::Range;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_where_bounds_runs_at_each_failing_item() {
        let items = [1, 1, 0, 1, 1, 1];
        assert_eq!(runs_where(&items, |&n| n == 1), vec![0..2, 3..6]);
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
}

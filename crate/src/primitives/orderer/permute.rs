//! Permutes a slot order by a `classify` closure, pinning every slot
//! the closure declines.

use std::ops::Range;

/// Convenience wrapper for `permute_in_place` over the full `items`
/// span. Shared by every caller that sorts the entire slice rather
/// than a sub-run.
pub(crate) fn permute_full<'a, T, K>(
    order: &mut [usize],
    items: &'a [T],
    classify: impl FnMut(&'a T) -> Option<K>,
) -> bool
where
    K: Ord,
{
    permute_in_place(order, items, 0..items.len(), classify)
}

/// Permutes the slots of `order` within `range` in place by sorting
/// items classified as `Some(K)`. Items returning `None` pin in their
/// current slot. Stable across equal keys. Returns `true` when the
/// permutation actually rewrote any slot.
pub(crate) fn permute_in_place<'a, T, K>(
    order: &mut [usize],
    items: &'a [T],
    range: Range<usize>,
    mut classify: impl FnMut(&'a T) -> Option<K>,
) -> bool
where
    K: Ord,
{
    let (slots, mut keyed): (Vec<usize>, Vec<(K, usize)>) = range
        .filter_map(|slot| {
            let src = order[slot];
            classify(&items[src]).map(|k| (slot, (k, src)))
        })
        .unzip();
    if keyed.is_sorted_by_key(|x| &x.0) {
        return false;
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    for (slot, (_, src)) in slots.into_iter().zip(keyed) {
        order[slot] = src;
    }
    true
}

/// Permutes the slots within each run of `runs` independently, items
/// classified `None` pinning in place. Returns `true` when any run rewrote a
/// slot. The many-run counterpart to [`permute_full`], keeping each sort
/// within its run so no item crosses a boundary.
pub(crate) fn permute_runs<'a, T, K>(
    order: &mut [usize],
    items: &'a [T],
    runs: impl IntoIterator<Item = Range<usize>>,
    mut classify: impl FnMut(&'a T) -> Option<K>,
) -> bool
where
    K: Ord,
{
    runs.into_iter().fold(false, |permuted, run| {
        permuted | permute_in_place(order, items, run, &mut classify)
    })
}

/// True when `order` is the identity permutation `0..order.len()`, the
/// signal a reorder left every slot in source position.
pub(super) fn is_identity(order: &[usize]) -> bool {
    order.iter().copied().eq(0..order.len())
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(&[0, 1, 2], true)]
    #[case(&[0, 2, 1], false)]
    #[case(&[], true)]
    fn is_identity_detects_the_identity_permutation(
        #[case] order: &[usize],
        #[case] expected: bool,
    ) {
        assert_eq!(is_identity(order), expected);
    }

    #[test]
    fn permute_in_place_leaves_slots_outside_range_untouched() {
        let mut order = vec![0, 1, 2, 3];
        let items = ["d", "c", "b", "a"];
        let permuted = permute_in_place(&mut order, &items, 1..3, |s: &&str| Some(*s));
        assert!(permuted);
        assert_eq!(order, vec![0, 2, 1, 3]);
    }

    #[test]
    fn permute_in_place_pins_unclassified() {
        let mut order = vec![0, 1, 2];
        let items = ["b", "x", "a"];
        let permuted = permute_in_place(&mut order, &items, 0..3, |s: &&str| {
            (*s != "x").then_some(*s)
        });
        assert!(permuted);
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn permute_in_place_preserves_relative_order_of_equal_keys() {
        let mut order = vec![0, 1, 2, 3];
        let items = [(2, 'a'), (1, 'b'), (1, 'c'), (1, 'd')];
        let permuted = permute_in_place(&mut order, &items, 0..4, |t: &(u8, char)| Some(t.0));
        assert!(permuted);
        assert_eq!(order, vec![1, 2, 3, 0]);
    }

    #[test]
    fn permute_in_place_returns_false_when_already_sorted() {
        let mut order = vec![0, 1, 2];
        let items = ["a", "b", "c"];
        let permuted = permute_in_place(&mut order, &items, 0..3, |s: &&str| Some(*s));
        assert!(!permuted);
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn permute_in_place_returns_false_when_fewer_than_two_classified() {
        let mut order = vec![0, 1, 2];
        let items = ["x", "x", "a"];
        let permuted = permute_in_place(&mut order, &items, 0..3, |s: &&str| {
            (*s != "x").then_some(*s)
        });
        assert!(!permuted);
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn permute_runs_returns_false_when_no_run_reorders() {
        let mut order = vec![0, 1, 2];
        let items = ["a", "b", "c"];
        let permuted = permute_runs(&mut order, &items, [0..1, 1..3], |s: &&str| Some(*s));
        assert!(!permuted);
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn permute_runs_sorts_each_run_without_crossing_a_boundary() {
        let mut order = vec![0, 1, 2, 3, 4];
        let items = ["b", "a", "z", "d", "c"];
        let permuted = permute_runs(&mut order, &items, [0..2, 3..5], |s: &&str| Some(*s));
        assert!(permuted);
        assert_eq!(order, vec![1, 0, 2, 4, 3]);
    }
}

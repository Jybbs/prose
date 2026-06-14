//! Line-distribution arithmetic for expanded collection layouts:
//! how many atomic items pack onto each line under the width and
//! `max-atomics` caps. A row's fit is tested against the widest items
//! of its size rather than the items in source order, so the
//! distribution holds whatever order `alphabetize` later imposes on
//! the elements.

use std::ops::Range;

/// Distributes items into the fewest equal-count lines whose widest
/// possible row fits `available` characters and whose count stays
/// within `max_atomics`. The fit test sums the widest items for a
/// row's size rather than the items in source order, so any later
/// reordering of the elements lands within budget on the same
/// distribution. Falls back to one item per line when no line count
/// fits, the floor a single over-wide item forces.
pub(super) fn flow_lines(
    widths: &[usize],
    available: usize,
    max_atomics: usize,
) -> Vec<Range<usize>> {
    let n = widths.len();
    if n == 0 {
        return Vec::new();
    }
    // Running sum of the widest `k` items, so a row of `k` items fits
    // under any permutation when `widest[k]` plus its separators clears
    // `available`.
    let mut descending = widths.to_vec();
    descending.sort_unstable_by(|a, b| b.cmp(a));
    let widest: Vec<usize> = std::iter::once(0)
        .chain(descending.iter().scan(0usize, |sum, &w| {
            *sum += w;
            Some(*sum)
        }))
        .collect();
    let fits = |num_lines: usize| {
        let row = n.div_ceil(num_lines);
        row <= max_atomics.max(1) && widest[row] + 2 * row.saturating_sub(1) <= available
    };
    let num_lines = (1..=n).find(|&num_lines| fits(num_lines)).unwrap_or(n);
    even_split(n, num_lines)
}

/// Splits `n` items into `num_lines` contiguous lines of near-equal
/// count, the first `n % num_lines` lines carrying one extra item.
fn even_split(n: usize, num_lines: usize) -> Vec<Range<usize>> {
    let base = n / num_lines;
    let remainder = n % num_lines;
    let mut lines = Vec::with_capacity(num_lines);
    let mut start = 0;
    for k in 0..num_lines {
        let end = start + base + usize::from(k < remainder);
        lines.push(start..end);
        start = end;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_split_distributes_remainder_to_leading_lines() {
        assert_eq!(even_split(5, 2), vec![0..3, 3..5]);
    }

    #[test]
    fn even_split_divides_evenly_without_remainder() {
        assert_eq!(even_split(6, 3), vec![0..2, 2..4, 4..6]);
    }

    #[test]
    fn flow_lines_escalates_when_an_even_split_would_overflow() {
        // Two lines would cluster three 10-wide items on one row past
        // available=23, so the packer escalates to three rows of two.
        assert_eq!(
            flow_lines(&[10, 10, 10, 1, 1, 1], 23, 8),
            vec![0..2, 2..4, 4..6]
        );
    }

    #[test]
    fn flow_lines_falls_back_to_one_per_line_when_no_split_fits() {
        // The lone 100-wide item clears no budget, so every row count
        // fails its fit and the fallback emits one item per line.
        assert_eq!(flow_lines(&[100], 10, 8), vec![0..1]);
    }

    #[test]
    fn flow_lines_is_independent_of_element_order() {
        let sorted = flow_lines(&[4, 4, 4, 32, 32, 32], 84, usize::MAX);
        let shuffled = flow_lines(&[32, 4, 32, 4, 32, 4], 84, usize::MAX);
        assert_eq!(sorted, shuffled);
        assert_eq!(sorted, vec![0..2, 2..4, 4..6]);
    }

    #[test]
    fn flow_lines_packs_into_one_line_when_budget_allows() {
        assert_eq!(flow_lines(&[1, 1, 1, 1], 80, 8), vec![0..4]);
    }

    #[test]
    fn flow_lines_returns_empty_for_empty_widths() {
        assert!(flow_lines(&[], 80, 8).is_empty());
    }

    #[test]
    fn flow_lines_splits_when_available_width_forces_it() {
        assert_eq!(flow_lines(&[10, 10, 10], 12, 8), vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn flow_lines_splits_when_max_atomics_forces_it() {
        assert_eq!(flow_lines(&[1; 6], 80, 3), vec![0..3, 3..6]);
    }
}

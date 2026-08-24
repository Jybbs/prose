//! Line-distribution arithmetic for expanded collection layouts:
//! how many atomic items pack onto each line under the width and
//! `max-atomics` caps, every row tested at the widths it carries once a
//! later sort has placed the entries.

use std::ops::Range;

/// The terms a flow segment packs under: the columns one row may fill,
/// the item cap per row, and whether an item follows the segment,
/// closing its last row with a comma as well.
#[derive(Clone, Copy)]
pub(super) struct Packing {
    pub(super) available: usize,
    pub(super) followed: bool,
    pub(super) max_atomics: usize,
}

/// Distributes items into the fewest equal-count lines that fit under
/// `packing`, charging every row the comma closing it wherever an item
/// follows, on a later row or past the segment. Falls back to one item
/// per line when no line count fits, the floor a single over-wide item
/// forces.
pub(super) fn flow_lines(widths: &[usize], packing: Packing) -> Vec<Range<usize>> {
    let n = widths.len();
    if n == 0 {
        return Vec::new();
    }
    let fits = |num_lines: usize| {
        let lines = even_split(n, num_lines);
        lines.iter().enumerate().all(|(k, line)| {
            let closes = k + 1 < lines.len() || packing.followed;
            let width: usize = widths[line.clone()].iter().sum();
            line.len() <= packing.max_atomics.max(1)
                && width + 2 * (line.len() - 1) + usize::from(closes) <= packing.available
        })
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
    use rstest::rstest;

    use super::*;

    /// A segment nothing follows.
    fn packing(available: usize, max_atomics: usize) -> Packing {
        Packing {
            available,
            followed: false,
            max_atomics,
        }
    }

    #[test]
    fn even_split_distributes_remainder_to_leading_lines() {
        assert_eq!(even_split(5, 2), vec![0..3, 3..5]);
    }

    #[test]
    fn even_split_divides_evenly_without_remainder() {
        assert_eq!(even_split(6, 3), vec![0..2, 2..4, 4..6]);
    }

    #[test]
    fn flow_lines_charges_the_separator_closing_a_row() {
        // Two rows of two would leave the first exactly on the budget
        // before its comma lands, so the packer escalates to three rows.
        assert_eq!(
            flow_lines(&[10, 10, 10], packing(22, 8)),
            vec![0..1, 1..2, 2..3]
        );
    }

    #[test]
    fn flow_lines_charges_the_comma_a_following_item_closes_the_last_row_with() {
        // Alone, the pair fills its single row exactly, whereas an item
        // past the segment closes that row with a comma the budget lacks.
        let alone = packing(22, 8);
        let followed = Packing {
            followed: true,
            ..alone
        };
        assert_eq!(flow_lines(&[10, 10], alone), vec![0..2]);
        assert_eq!(flow_lines(&[10, 10], followed), vec![0..1, 1..2]);
    }

    #[test]
    fn flow_lines_escalates_when_an_even_split_would_overflow() {
        // Two lines would cluster three 10-wide items on one row past
        // available=23, so the packer escalates to three rows of two.
        assert_eq!(
            flow_lines(&[10, 10, 10, 1, 1, 1], packing(23, 8)),
            vec![0..2, 2..4, 4..6]
        );
    }

    #[test]
    fn flow_lines_falls_back_to_one_per_line_when_no_split_fits() {
        // The lone 100-wide item clears no budget, so every row count
        // fails its fit and the fallback emits one item per line.
        assert_eq!(flow_lines(&[100], packing(10, 8)), vec![0..1]);
    }

    #[rstest]
    #[case::widest_pair_last(&[7, 7, 11, 11], vec![0..2, 2..4])]
    #[case::widest_pair_first(&[11, 11, 7, 7], vec![0..1, 1..2, 2..3, 3..4])]
    fn flow_lines_reads_the_run_row_by_row(
        #[case] widths: &[usize],
        #[case] expected: Vec<Range<usize>>,
    ) {
        // The widest pair reaches 24 columns before its comma, which only
        // a row an item follows carries, so the pair packs where the
        // order leaves it last and splits where a row follows it.
        assert_eq!(flow_lines(widths, packing(24, 8)), expected);
    }

    #[test]
    fn flow_lines_packs_into_one_line_when_budget_allows() {
        assert_eq!(flow_lines(&[1, 1, 1, 1], packing(80, 8)), vec![0..4]);
    }

    #[test]
    fn flow_lines_returns_empty_for_empty_widths() {
        assert!(flow_lines(&[], packing(80, 8)).is_empty());
    }

    #[test]
    fn flow_lines_splits_when_available_width_forces_it() {
        assert_eq!(
            flow_lines(&[10, 10, 10], packing(12, 8)),
            vec![0..1, 1..2, 2..3]
        );
    }

    #[test]
    fn flow_lines_splits_when_max_atomics_forces_it() {
        assert_eq!(flow_lines(&[1; 6], packing(80, 3)), vec![0..3, 3..6]);
    }
}

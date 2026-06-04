//! Line-distribution arithmetic for expanded collection layouts:
//! how many atomic items pack onto each line under the width and
//! `max-atomics` caps.

use std::ops::Range;

/// Distributes items into the smallest number of lines such that no
/// line exceeds `available` characters and no line carries more than
/// `max_atomics` items, giving roughly equal item counts per line.
/// Escalates to more lines if the even split at the minimum line
/// count would still overflow either cap.
pub(super) fn flow_lines(
    widths: &[usize],
    available: usize,
    max_atomics: usize,
) -> Vec<Range<usize>> {
    if widths.is_empty() {
        return Vec::new();
    }
    let n = widths.len();
    let prefix: Vec<usize> = std::iter::once(0)
        .chain(widths.iter().scan(0usize, |sum, &w| {
            *sum += w;
            Some(*sum)
        }))
        .collect();
    let total_slot = prefix[n] + 2 * n.saturating_sub(1);
    let by_width = total_slot.div_ceil(available.max(1));
    let by_cap = n.div_ceil(max_atomics.max(1));
    let initial = by_width.max(by_cap).max(1);
    (initial..=n)
        .find_map(|num_lines| try_even(&prefix, num_lines, available, max_atomics))
        .unwrap_or_else(|| (0..n).map(|i| i..(i + 1)).collect())
}

/// Attempts an even distribution of items across `num_lines` lines.
/// `prefix` is a length-`n+1` running sum of per-item widths so the
/// slot sum for any line `[start..end)` is one subtraction. Returns
/// `None` when any line would exceed `max_atomics` items or
/// `available` slot width. First `n % num_lines` lines carry one
/// extra item.
fn try_even(
    prefix: &[usize],
    num_lines: usize,
    available: usize,
    max_atomics: usize,
) -> Option<Vec<Range<usize>>> {
    let n = prefix.len() - 1;
    if num_lines == 0 || num_lines > n {
        return None;
    }
    let base = n / num_lines;
    let remainder = n % num_lines;
    let mut lines = Vec::with_capacity(num_lines);
    let mut start = 0;
    for k in 0..num_lines {
        let size = base + usize::from(k < remainder);
        if size > max_atomics {
            return None;
        }
        let end = start + size;
        let slot = prefix[end] - prefix[start] + 2 * size.saturating_sub(1);
        if slot > available {
            return None;
        }
        lines.push(start..end);
        start = end;
    }
    Some(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_lines_escalates_when_initial_split_overflows() {
        // Total width forces an initial guess of 2 lines, but the
        // only even 2-line split puts three 10-wide items on one line
        // and overflows. The find_map must escalate to 3 lines.
        let lines = flow_lines(&[10, 10, 10, 1, 1, 1], 23, 8);
        assert_eq!(lines, vec![0..2, 2..4, 4..6]);
    }

    #[test]
    fn flow_lines_falls_back_to_one_per_line_when_no_split_fits() {
        // The lone 100-wide item exceeds available=10, pushing initial
        // above n=1 so the find_map range is empty. The fallback
        // emits one item per line.
        let lines = flow_lines(&[100], 10, 8);
        assert_eq!(lines, vec![0..1]);
    }

    #[test]
    fn flow_lines_packs_into_one_line_when_budget_allows() {
        let lines = flow_lines(&[1, 1, 1, 1], 80, 8);
        assert_eq!(lines, vec![0..4]);
    }

    #[test]
    fn flow_lines_returns_empty_for_empty_widths() {
        assert!(flow_lines(&[], 80, 8).is_empty());
    }

    #[test]
    fn flow_lines_splits_when_available_width_forces_it() {
        let lines = flow_lines(&[10, 10, 10], 12, 8);
        assert_eq!(lines, vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn flow_lines_splits_when_max_atomics_forces_it() {
        let lines = flow_lines(&[1; 6], 80, 3);
        assert_eq!(lines, vec![0..3, 3..6]);
    }

    #[test]
    fn try_even_distributes_remainder_to_leading_lines() {
        let lines = try_even(&[0, 1, 2, 3, 4, 5], 2, 80, 8).expect("split fits");
        assert_eq!(lines, vec![0..3, 3..5]);
    }

    #[test]
    fn try_even_rejects_more_lines_than_items() {
        assert!(try_even(&[0, 1, 2], 3, 80, 8).is_none());
    }

    #[test]
    fn try_even_rejects_zero_lines() {
        assert!(try_even(&[0, 1, 2], 0, 80, 8).is_none());
    }

    #[test]
    fn try_even_returns_none_when_max_atomics_exceeded() {
        assert!(try_even(&[0, 1, 2, 3, 4, 5], 1, 80, 3).is_none());
    }

    #[test]
    fn try_even_returns_none_when_slot_overflows() {
        assert!(try_even(&[0, 50, 100], 1, 60, 8).is_none());
    }
}

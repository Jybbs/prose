//! Computes per-line padding widths for alignment rules.
//!
//! `compute` is a stateless helper for the alignment rules that do
//! not track group min/max themselves: `align_colons`,
//! `align_imports`, and `match_case_align`. `align_equals` inlines
//! the padding math because its shift-limit policy already carries
//! min and max in hand, so calling `compute` would re-scan for the
//! max. Group boundaries stay rule-specific, leaving only the
//! padding math here.

/// Returns the per-line padding widths that align a shared token.
///
/// `widths` is the display width of each row's left-hand-side region
/// (the text preceding the shared token). The target column is
/// `max(widths)` and each returned padding is `target - widths[i]`,
/// leaving zero padding on the widest row. Callers add any
/// post-target spacing (typically one space) themselves.
///
/// Returns an empty `Vec` for empty input and `vec![0]` for a single
/// item, leaving the singleton rule free to apply its own spacing.
pub fn compute(widths: &[usize]) -> Vec<usize> {
    let target = widths.iter().copied().max().unwrap_or(0);
    widths.iter().map(|&w| target - w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascending_widths_pad_shorter_to_longest() {
        assert_eq!(compute(&[1, 3, 2]), vec![2, 0, 1]);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(compute(&[]), Vec::<usize>::new());
    }

    #[test]
    fn identical_widths_all_zero() {
        assert_eq!(compute(&[2, 2, 2]), vec![0, 0, 0]);
    }

    #[test]
    fn single_item_returns_zero() {
        assert_eq!(compute(&[5]), vec![0]);
    }
}

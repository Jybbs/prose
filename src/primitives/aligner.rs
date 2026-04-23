//! Computes per-line padding widths for alignment rules.
//!
//! `compute` is a stateless helper shared by `align_equals`,
//! `align_colons`, `align_imports`, and `match_case_align`. Group
//! boundaries stay rule-specific; only the padding math lives here.

use unicode_width::UnicodeWidthStr;

/// Returns the per-line padding widths that align a shared token.
///
/// The target column is `max(width(s))` across `befores`. Each returned
/// padding is `target - width(s)`, leaving zero padding on the longest
/// line. Widths use `unicode-width` conventions, so zero-width combining
/// marks, CJK full-width, and East Asian ambiguous characters all
/// contribute correctly.
///
/// Returns an empty `Vec` for empty input and `vec![0]` for a single
/// item, leaving the singleton rule free to apply its own spacing.
pub fn compute(befores: &[&str]) -> Vec<usize> {
    let target = befores.iter().map(|s| s.width()).max().unwrap_or(0);
    befores.iter().map(|s| target - s.width()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_pads_shorter_to_longest() {
        assert_eq!(compute(&["a", "abc", "ab"]), vec![2, 0, 1]);
    }

    #[test]
    fn cjk_counts_as_double_width() {
        assert_eq!(compute(&["中", "ab", "a"]), vec![0, 0, 1]);
    }

    #[test]
    fn combining_marks_are_zero_width() {
        assert_eq!(compute(&["a\u{0301}", "ab"]), vec![1, 0]);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(compute(&[]), Vec::<usize>::new());
    }

    #[test]
    fn identical_widths_all_zero() {
        assert_eq!(compute(&["ab", "cd", "ef"]), vec![0, 0, 0]);
    }

    #[test]
    fn single_item_returns_zero() {
        assert_eq!(compute(&["x"]), vec![0]);
    }
}

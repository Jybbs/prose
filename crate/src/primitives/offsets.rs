//! Source-offset primitive, the start of the whitespace run preceding
//! an offset.

use ruff_text_size::TextSize;

/// Returns the start of the contiguous ASCII-whitespace run immediately
/// preceding `offset` in `text`.
pub(crate) fn whitespace_start_before(text: &str, offset: TextSize) -> TextSize {
    let trimmed = text[..offset.to_usize()].trim_end_matches(|c: char| c.is_ascii_whitespace());
    TextSize::of(trimmed)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::crlf("a\r\n\r\nb", 5, 1)]
    #[case::leading_whitespace("   \n\n\nx", 6, 0)]
    #[case::stops_at_non_whitespace("ab\n\ncd", 4, 2)]
    fn whitespace_start_before_walks_back_over_the_run(
        #[case] text: &str,
        #[case] offset: u32,
        #[case] expected: u32,
    ) {
        assert_eq!(
            whitespace_start_before(text, TextSize::new(offset)),
            TextSize::new(expected),
        );
    }
}

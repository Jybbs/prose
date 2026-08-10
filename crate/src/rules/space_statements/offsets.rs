//! Source-offset primitive for blank-line normalization, the start of
//! the whitespace run preceding an offset.

use ruff_text_size::TextSize;

/// Returns the start of the contiguous ASCII-whitespace run immediately
/// preceding `offset` in `text`.
pub(super) fn whitespace_start_before(text: &str, offset: TextSize) -> TextSize {
    let trimmed = text[..offset.to_usize()].trim_end_matches(|c: char| c.is_ascii_whitespace());
    TextSize::of(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_start_before_handles_crlf() {
        assert_eq!(
            whitespace_start_before("a\r\n\r\nb", TextSize::new(5)),
            TextSize::new(1),
        );
    }

    #[test]
    fn whitespace_start_before_returns_zero_for_leading_whitespace() {
        assert_eq!(
            whitespace_start_before("   \n\n\nx", TextSize::new(6)),
            TextSize::new(0),
        );
    }

    #[test]
    fn whitespace_start_before_stops_at_non_whitespace() {
        assert_eq!(
            whitespace_start_before("ab\n\ncd", TextSize::new(4)),
            TextSize::new(2),
        );
    }
}

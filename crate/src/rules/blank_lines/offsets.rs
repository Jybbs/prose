//! Source-offset primitives for blank-line normalization: the end of
//! a header signature and the start of a whitespace run.

use ruff_text_size::{Ranged, TextSize};

use crate::source::Source;

/// Returns the position immediately after the `:` that introduces a
/// class or function body whose first statement starts at `body_start`.
/// Scans backward from `body_start` through whitespace and comments,
/// landing on the first non-trivia token. Falls back to `body_start`
/// when the scan finds none.
pub(super) fn header_signature_end(source: &Source, body_start: TextSize) -> TextSize {
    source
        .prev_non_trivia_token(body_start)
        .map_or(body_start, |t| t.end())
}

/// Returns the start of the contiguous ASCII-whitespace run immediately
/// preceding `offset` in `text`.
pub(super) fn whitespace_start_before(text: &str, offset: TextSize) -> TextSize {
    let trimmed = text[..offset.to_usize()].trim_end_matches(|c: char| c.is_ascii_whitespace());
    TextSize::of(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{first_class, first_def, parse};

    #[test]
    fn header_signature_end_handles_multi_line_function_signature() {
        let s = parse("def f(\n    x,\n    y,\n):\n    pass\n");
        let func = first_def(&s);
        let end = header_signature_end(&s, func.body[0].start());
        assert!(s.text()[..end.to_usize()].ends_with("):"));
    }

    #[test]
    fn header_signature_end_points_after_colon_in_simple_class() {
        let s = parse("class C:\n    pass\n");
        let class = first_class(&s);
        let end = header_signature_end(&s, class.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "class C:");
    }

    #[test]
    fn header_signature_end_points_after_colon_in_simple_function() {
        let s = parse("def f():\n    pass\n");
        let func = first_def(&s);
        let end = header_signature_end(&s, func.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "def f():");
    }

    #[test]
    fn header_signature_end_skips_eol_comment_on_header_line() {
        let s = parse("class C:  # eol\n    pass\n");
        let class = first_class(&s);
        let end = header_signature_end(&s, class.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "class C:");
    }

    #[test]
    fn header_signature_end_skips_own_line_comment_above_body() {
        let s = parse("class C:\n    # comment\n    pass\n");
        let class = first_class(&s);
        let end = header_signature_end(&s, class.body[0].start());
        assert_eq!(&s.text()[..end.to_usize()], "class C:");
    }

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

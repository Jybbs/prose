//! Edit-shaping primitives shared across rules. `narrow_edit` trims
//! a candidate replacement to its minimal divergent range against
//! the source. `apply_inline_edits` folds a list of edits into a
//! source range, returning `Cow::Borrowed` when no edit applies.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::source::Source;

/// Folds any leaf edits whose range falls inside `range` into the
/// source slice for that range. Returns `Cow::Borrowed` when no leaf
/// edit applies.
pub(crate) fn apply_inline_edits<'src>(
    source: &'src Source,
    range: TextRange,
    edits: &[Edit],
) -> Cow<'src, str> {
    let mut inside: Vec<&Edit> = edits
        .iter()
        .filter(|e| range.contains_range(e.range()))
        .collect();
    if inside.is_empty() {
        return Cow::Borrowed(source.slice(range));
    }
    inside.sort();
    let mut out = String::with_capacity(range.len().to_usize());
    let mut cursor = range.start();
    for edit in inside {
        out.push_str(source.slice(TextRange::new(cursor, edit.range().start())));
        out.push_str(edit.content().unwrap_or_default());
        cursor = edit.range().end();
    }
    out.push_str(source.slice(TextRange::new(cursor, range.end())));
    Cow::Owned(out)
}

/// Trims a candidate replacement to its minimal spanning range by
/// stripping the longest common codepoint prefix and suffix shared
/// with `source_slice`. Returns `None` when `text` already equals
/// `source_slice` (no edit needed). Walks codepoint-by-codepoint so
/// the trim never lands inside a multibyte UTF-8 sequence.
pub(crate) fn narrow_edit(
    mut text: String,
    span: TextRange,
    source_slice: &str,
) -> Option<(TextRange, String)> {
    if text == source_slice {
        return None;
    }
    let prefix_len: TextSize = text
        .chars()
        .zip(source_slice.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.text_len())
        .sum();
    let prefix_bytes = prefix_len.to_usize();
    let text_tail = &text[prefix_bytes..];
    let source_tail = &source_slice[prefix_bytes..];
    let suffix_len: TextSize = text_tail
        .chars()
        .rev()
        .zip(source_tail.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.text_len())
        .sum();
    let suffix_bytes = suffix_len.to_usize();
    text.truncate(text.len() - suffix_bytes);
    text.drain(..prefix_bytes);
    Some((span.add_start(prefix_len).sub_end(suffix_len), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_edit_handles_multibyte_codepoint_at_divergence() {
        let span = TextRange::new(0u32.into(), 7u32.into());
        let (range, text) =
            narrow_edit("α = 1\n".to_owned(), span, "β = 1\n").expect("differs");
        assert_eq!(range.start().to_u32(), 0);
        assert_eq!(range.end().to_u32(), 2);
        assert_eq!(text, "α");
    }

    #[test]
    fn narrow_edit_handles_pure_deletion() {
        let span = TextRange::new(0u32.into(), 3u32.into());
        let (range, text) = narrow_edit("ab".to_owned(), span, "abc").expect("differs");
        assert_eq!(range.start().to_u32(), 2);
        assert_eq!(range.end().to_u32(), 3);
        assert_eq!(text, "");
    }

    #[test]
    fn narrow_edit_handles_pure_insertion() {
        let span = TextRange::new(0u32.into(), 3u32.into());
        let (range, text) = narrow_edit("abxc".to_owned(), span, "abc").expect("differs");
        assert_eq!(range.start().to_u32(), 2);
        assert_eq!(range.end().to_u32(), 2);
        assert_eq!(text, "x");
    }

    #[test]
    fn narrow_edit_returns_none_when_text_equals_source() {
        let span = TextRange::new(0u32.into(), 5u32.into());
        assert!(narrow_edit("hello".to_owned(), span, "hello").is_none());
    }

    #[test]
    fn narrow_edit_returns_whole_input_when_no_common_prefix_or_suffix() {
        let span = TextRange::new(0u32.into(), 3u32.into());
        let (range, text) = narrow_edit("abc".to_owned(), span, "xyz").expect("differs");
        assert_eq!(range.start().to_u32(), 0);
        assert_eq!(range.end().to_u32(), 3);
        assert_eq!(text, "abc");
    }

    #[test]
    fn narrow_edit_trims_common_prefix() {
        let span = TextRange::new(0u32.into(), 3u32.into());
        let (range, text) = narrow_edit("abc".to_owned(), span, "abd").expect("differs");
        assert_eq!(range.start().to_u32(), 2);
        assert_eq!(range.end().to_u32(), 3);
        assert_eq!(text, "c");
    }

    #[test]
    fn narrow_edit_trims_common_prefix_and_suffix() {
        let span = TextRange::new(0u32.into(), 7u32.into());
        let (range, text) =
            narrow_edit("ab1cdef".to_owned(), span, "ab2cdef").expect("differs");
        assert_eq!(range.start().to_u32(), 2);
        assert_eq!(range.end().to_u32(), 3);
        assert_eq!(text, "1");
    }

    #[test]
    fn narrow_edit_trims_common_suffix() {
        let span = TextRange::new(0u32.into(), 3u32.into());
        let (range, text) = narrow_edit("abc".to_owned(), span, "xbc").expect("differs");
        assert_eq!(range.start().to_u32(), 0);
        assert_eq!(range.end().to_u32(), 1);
        assert_eq!(text, "a");
    }
}

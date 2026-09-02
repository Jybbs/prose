//! Maps a source offset, a range, an edit, and a notebook's cell
//! boundaries through the `SourceMap` of an applied edit set, and
//! narrows a whole-range replacement to the span that actually differs.

use std::{borrow::Cow, cmp::Ordering};

use ruff_diagnostics::{Edit, SourceMap, SourceMarker};
use ruff_notebook::CellOffsets;
use ruff_text_size::{TextLen, TextRange, TextSize};

use super::*;
use crate::source::Source;

/// Forwards each cell boundary in `offsets` through `map`, shifting it
/// by the delta of the nearest marker at or before it, the slide that
/// keeps notebook cell boundaries current across a reparse. `limit` is
/// the length of the text the forwarded offsets describe, and every
/// boundary lands inside it and at or after the boundary before it, so
/// the result indexes that text and cuts its cells in order.
pub(crate) fn forward_offsets(
    offsets: &CellOffsets,
    map: &SourceMap,
    limit: TextSize,
) -> CellOffsets {
    let mut forwarded = offsets.clone();
    let last = forwarded.len().saturating_sub(1);
    let mut floor = TextSize::default();
    for (i, offset) in forwarded.iter_mut().enumerate() {
        *offset = forward_offset(*offset, map, i == last).clamp(floor, limit);
        floor = *offset;
    }
    forwarded
}

/// `range` at the position the woven text `map` describes carries it,
/// `None` where an edit in `map` replaced either end of it.
pub(crate) fn forward_range(range: TextRange, map: &SourceMap) -> Option<TextRange> {
    let start = forward_start(range.start(), map)?;
    let end = forward_end(range.end(), map)?;
    (start <= end).then(|| TextRange::new(start, end))
}

/// `offset`, a token's first byte, at the position the woven text
/// `map` describes carries it, `None` where an edit in `map` replaced
/// the token. An insertion at the offset lands ahead of the token.
pub(crate) fn forward_start(offset: TextSize, map: &SourceMap) -> Option<TextSize> {
    let markers = map.markers();
    forward_through(
        offset,
        markers,
        markers.partition_point(|marker| marker.source() <= offset),
    )
}

/// Narrows `text` against the source slice covered by `span` and
/// shapes the result as either a deletion or replacement Edit.
/// Returns `None` when the text already matches the source slice.
pub(crate) fn narrowed_replacement<'a>(
    source: &Source,
    span: TextRange,
    text: impl Into<Cow<'a, str>>,
) -> Option<Edit> {
    let (narrowed_span, narrowed_text) = narrow_edit(text.into(), span, source.slice(span))?;
    Some(replacement_or_deletion(narrowed_span, narrowed_text))
}

/// `offset` moved by the delta of the last marker at or before it,
/// left where it is when no marker precedes it, the slide a reparse
/// reads for a range no edit replaced.
pub(crate) fn shifted_past(offset: TextSize, markers: &[SourceMarker]) -> TextSize {
    if let Some(last) = markers.last()
        && last.source() <= offset
    {
        return shifted(offset, last);
    }
    let upto = markers.partition_point(|marker| marker.source() <= offset);
    shifted_by_last(offset, &markers[..upto])
}

/// `offset`, the byte past a token, at the position the woven text
/// `map` describes carries it, `None` where an edit in `map` replaced
/// the token. An insertion at the offset lands past the token.
fn forward_end(offset: TextSize, map: &SourceMap) -> Option<TextSize> {
    let markers = map.markers();
    forward_through(
        offset,
        markers,
        markers.partition_point(|marker| marker.source() < offset),
    )
}

/// Shifts a single offset by the delta of the nearest marker at or
/// before it, the per-boundary slide [`forward_offsets`] maps over a
/// notebook's cell offsets. Markers sharing an interior offset's exact
/// source resolve to the first pushed, so an insertion landing on a
/// cell boundary stays inside the cell it opens, whereas the final
/// boundary resolves to the last pushed, keeping an end-of-buffer
/// insertion inside the last cell.
fn forward_offset(offset: TextSize, map: &SourceMap, is_final: bool) -> TextSize {
    let markers = map.markers();
    if is_final {
        return shifted_past(offset, markers);
    }
    let index = markers.partition_point(|marker| marker.source() < offset);
    if let Some(marker) = markers
        .get(index)
        .filter(|marker| marker.source() == offset)
    {
        return shifted(offset, marker);
    }
    if !index.is_multiple_of(2) {
        return markers[index - 1].dest();
    }
    shifted_by_last(offset, &markers[..index])
}

/// `offset` shifted by the last of the first `passed` markers, or
/// `None` where that count is odd, the last marker then opening an
/// edit that closes past the offset and replaces the byte the caller
/// named.
fn forward_through(offset: TextSize, markers: &[SourceMarker], passed: usize) -> Option<TextSize> {
    passed
        .is_multiple_of(2)
        .then(|| shifted_by_last(offset, &markers[..passed]))
}

/// Trims a candidate replacement to its minimal spanning range by
/// stripping the longest common codepoint prefix and suffix shared
/// with `source_slice`. Returns `None` when `text` already equals
/// `source_slice` (no edit needed). Walks codepoint-by-codepoint so
/// the trim never lands inside a multibyte UTF-8 sequence.
fn narrow_edit(
    text: Cow<'_, str>,
    span: TextRange,
    source_slice: &str,
) -> Option<(TextRange, String)> {
    if text == source_slice {
        return None;
    }
    let mut text = text.into_owned();
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

/// `offset` moved by `marker`'s source-to-destination delta.
fn shifted(offset: TextSize, marker: &SourceMarker) -> TextSize {
    match marker.source().cmp(&marker.dest()) {
        Ordering::Less => offset + (marker.dest() - marker.source()),
        Ordering::Greater => offset - (marker.source() - marker.dest()),
        Ordering::Equal => offset,
    }
}

/// `offset` shifted by the last of `markers`, unchanged where there is
/// none.
fn shifted_by_last(offset: TextSize, markers: &[SourceMarker]) -> TextSize {
    markers
        .last()
        .map_or(offset, |marker| shifted(offset, marker))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{notebook, range, replacement, woven};

    /// The map of `edits` woven into `abcdef`.
    fn mapped(edits: Vec<Edit>) -> SourceMap {
        woven("abcdef", edits).1
    }

    #[test]
    fn forward_end_lands_ahead_of_an_insertion_at_the_offset() {
        let map = mapped(vec![Edit::insertion("XX".to_owned(), TextSize::new(2))]);

        assert_eq!(forward_end(TextSize::new(2), &map), Some(TextSize::new(2)));
    }

    #[rstest]
    #[case::ahead_of_every_edit(1, Some(1))]
    #[case::at_the_replaced_span_start(2, Some(2))]
    #[case::inside_the_replaced_span(3, None)]
    #[case::at_the_replaced_span_end(4, None)]
    #[case::past_the_replaced_span(5, Some(4))]
    fn forward_end_moves_the_byte_past_a_token(#[case] offset: u32, #[case] expected: Option<u32>) {
        let map = mapped(vec![Edit::range_replacement("X".to_owned(), range(2, 4))]);

        assert_eq!(
            forward_end(TextSize::new(offset), &map),
            expected.map(TextSize::new),
        );
    }

    #[test]
    fn forward_offset_lands_a_boundary_inside_a_replacement_at_its_start() {
        let (text, map) = woven("abcdef", vec![replacement("X", 1, 5)]);

        assert_eq!(text, "aXf");
        assert_eq!(
            forward_offset(TextSize::new(3), &map, false),
            TextSize::new(1)
        );
    }

    #[test]
    fn forward_offset_leaves_an_offset_before_every_marker() {
        let (_text, map) = woven("abcdef", vec![Edit::insertion("X".to_owned(), 3u32.into())]);

        assert_eq!(
            forward_offset(TextSize::new(1), &map, false),
            TextSize::new(1)
        );
    }

    #[test]
    fn forward_offset_leaves_an_offset_past_a_length_preserving_edit() {
        let (text, map) = woven("abc", vec![replacement("X", 0, 1)]);

        assert_eq!(text, "Xbc");
        assert_eq!(
            forward_offset(TextSize::new(2), &map, false),
            TextSize::new(2)
        );
    }

    #[rstest]
    #[case::ahead_of_it(1, 1)]
    #[case::at_the_insertion(2, 2)]
    #[case::past_it(4, 6)]
    fn forward_offset_moves_a_boundary_around_an_insertion(
        #[case] offset: u32,
        #[case] expected: u32,
    ) {
        let (text, map) = woven(
            "abcdef",
            vec![Edit::insertion("XX".to_owned(), 2u32.into())],
        );

        assert_eq!(text, "abXXcdef");
        assert_eq!(
            forward_offset(TextSize::new(offset), &map, false),
            TextSize::new(expected)
        );
    }

    #[test]
    fn forward_offset_slides_a_boundary_back_over_a_deletion() {
        let (text, map) = woven("abcdef", vec![Edit::range_deletion(range(1, 3))]);

        assert_eq!(text, "adef");
        assert_eq!(
            forward_offset(TextSize::new(0), &map, false),
            TextSize::new(0)
        );
        assert_eq!(
            forward_offset(TextSize::new(5), &map, false),
            TextSize::new(3)
        );
    }

    #[test]
    fn forward_offset_slides_the_final_boundary_past_an_end_insertion() {
        let (text, map) = woven("abc", vec![Edit::insertion("XX".to_owned(), 3u32.into())]);

        assert_eq!(text, "abcXX");
        assert_eq!(
            forward_offset(TextSize::new(3), &map, true),
            TextSize::new(5)
        );
        assert_eq!(
            forward_offset(TextSize::new(3), &map, false),
            TextSize::new(3)
        );
    }

    #[test]
    fn forward_offsets_holds_every_boundary_inside_the_shortened_text() {
        let source = notebook(&["x = 1\n", "y = 2\n"]);
        let (text, map) = woven(source.text(), vec![Edit::range_deletion(range(1, 11))]);
        let limit = text.text_len();

        let forwarded = forward_offsets(source.cell_offsets(), &map, limit);

        assert!(forwarded.iter().all(|offset| *offset <= limit));
        assert!(forwarded.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn forward_range_answers_none_for_an_empty_range_an_insertion_splits() {
        let map = mapped(vec![Edit::insertion("XX".to_owned(), TextSize::new(2))]);

        assert_eq!(forward_range(range(2, 2), &map), None);
        assert_eq!(forward_range(range(1, 2), &map), Some(range(1, 2)));
        assert_eq!(forward_range(range(2, 3), &map), Some(range(4, 5)));
    }

    #[rstest]
    #[case::spanning_an_interior_edit(0, 6, Some((0, 5)))]
    #[case::ending_at_the_replaced_span(0, 3, None)]
    #[case::opening_at_the_replaced_span(2, 6, None)]
    #[case::past_the_replaced_span(4, 6, Some((3, 5)))]
    fn forward_range_moves_both_ends(
        #[case] start: u32,
        #[case] end: u32,
        #[case] expected: Option<(u32, u32)>,
    ) {
        let map = mapped(vec![Edit::range_replacement("X".to_owned(), range(2, 4))]);

        assert_eq!(
            forward_range(range(start, end), &map),
            expected.map(|(start, end)| range(start, end)),
        );
    }

    #[test]
    fn forward_start_lands_past_an_insertion_at_the_offset() {
        let map = mapped(vec![Edit::insertion("XX".to_owned(), TextSize::new(2))]);

        assert_eq!(
            forward_start(TextSize::new(2), &map),
            Some(TextSize::new(4))
        );
    }

    #[rstest]
    #[case::ahead_of_every_edit(1, Some(1))]
    #[case::at_the_replaced_span_start(2, None)]
    #[case::inside_the_replaced_span(3, None)]
    #[case::at_the_replaced_span_end(4, Some(3))]
    #[case::past_the_replaced_span(5, Some(4))]
    fn forward_start_moves_a_tokens_first_byte(#[case] offset: u32, #[case] expected: Option<u32>) {
        let map = mapped(vec![Edit::range_replacement("X".to_owned(), range(2, 4))]);

        assert_eq!(
            forward_start(TextSize::new(offset), &map),
            expected.map(TextSize::new),
        );
    }

    #[test]
    fn forward_start_slides_a_token_back_over_a_deletion() {
        let map = mapped(vec![Edit::range_deletion(range(1, 3))]);

        assert_eq!(
            forward_start(TextSize::new(3), &map),
            Some(TextSize::new(1))
        );
        assert_eq!(forward_start(TextSize::new(1), &map), None);
    }

    #[test]
    fn narrow_edit_handles_multibyte_codepoint_at_divergence() {
        let span = range(0, 7);
        let (r, text) = narrow_edit("α = 1\n".into(), span, "β = 1\n").expect("differs");
        assert_eq!(r.start().to_u32(), 0);
        assert_eq!(r.end().to_u32(), 2);
        assert_eq!(text, "α");
    }

    #[test]
    fn narrow_edit_handles_pure_deletion() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("ab".into(), span, "abc").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "");
    }

    #[test]
    fn narrow_edit_handles_pure_insertion() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abxc".into(), span, "abc").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 2);
        assert_eq!(text, "x");
    }

    #[test]
    fn narrow_edit_returns_none_when_text_equals_source() {
        assert!(narrow_edit("hello".into(), range(0, 5), "hello").is_none());
    }

    #[test]
    fn narrow_edit_returns_whole_input_when_no_common_prefix_or_suffix() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abc".into(), span, "xyz").expect("differs");
        assert_eq!(r.start().to_u32(), 0);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "abc");
    }

    #[test]
    fn narrow_edit_trims_common_prefix() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abc".into(), span, "abd").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "c");
    }

    #[test]
    fn narrow_edit_trims_common_prefix_and_suffix() {
        let span = range(0, 7);
        let (r, text) = narrow_edit("ab1cdef".into(), span, "ab2cdef").expect("differs");
        assert_eq!(r.start().to_u32(), 2);
        assert_eq!(r.end().to_u32(), 3);
        assert_eq!(text, "1");
    }

    #[test]
    fn narrow_edit_trims_common_suffix() {
        let span = range(0, 3);
        let (r, text) = narrow_edit("abc".into(), span, "xbc").expect("differs");
        assert_eq!(r.start().to_u32(), 0);
        assert_eq!(r.end().to_u32(), 1);
        assert_eq!(text, "a");
    }
}

//! Maps a source offset through a set of edits, and narrows a whole-range
//! replacement to the span that actually differs.

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

/// Trims a candidate replacement to its minimal spanning range by
/// stripping the longest common codepoint prefix and suffix shared
/// with `source_slice`. Returns `None` when `text` already equals
/// `source_slice` (no edit needed). Walks codepoint-by-codepoint so
/// the trim never lands inside a multibyte UTF-8 sequence.
pub(super) fn narrow_edit(
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

/// `offset` moved by `marker`'s source-to-destination delta.
pub(super) fn shifted(offset: TextSize, marker: &SourceMarker) -> TextSize {
    match marker.source().cmp(&marker.dest()) {
        Ordering::Less => offset + (marker.dest() - marker.source()),
        Ordering::Greater => offset - (marker.source() - marker.dest()),
        Ordering::Equal => offset,
    }
}

/// `offset` moved by the delta of the last marker at or before it, left
/// where it is when no marker precedes it.
pub(crate) fn shifted_past(offset: TextSize, markers: &[SourceMarker]) -> TextSize {
    if let Some(last) = markers.last()
        && last.source() <= offset
    {
        return shifted(offset, last);
    }
    let upto = markers.partition_point(|marker| marker.source() <= offset);
    markers[..upto]
        .last()
        .map_or(offset, |marker| shifted(offset, marker))
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
    let Some(marker) = markers[..index].last() else {
        return offset;
    };
    if index % 2 == 1 {
        return marker.dest();
    }
    shifted(offset, marker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{notebook, range};

    #[test]
    fn forward_offset_keeps_a_boundary_before_an_insertion_at_it() {
        let (text, map) = apply_edits_mapped(
            "abcdef",
            vec![Edit::insertion("XX".to_owned(), 2u32.into())],
        )
        .expect("woven");

        assert_eq!(text, "abXXcdef");
        assert_eq!(
            forward_offset(TextSize::new(2), &map, false),
            TextSize::new(2)
        );
    }

    #[test]
    fn forward_offset_lands_a_boundary_inside_a_replacement_at_its_start() {
        let (text, map) = apply_edits_mapped(
            "abcdef",
            vec![Edit::range_replacement("X".to_owned(), range(1, 5))],
        )
        .expect("woven");

        assert_eq!(text, "aXf");
        assert_eq!(
            forward_offset(TextSize::new(3), &map, false),
            TextSize::new(1)
        );
    }

    #[test]
    fn forward_offset_leaves_an_offset_before_every_marker() {
        let (_text, map) =
            apply_edits_mapped("abcdef", vec![Edit::insertion("X".to_owned(), 3u32.into())])
                .expect("woven");

        assert_eq!(
            forward_offset(TextSize::new(1), &map, false),
            TextSize::new(1)
        );
    }

    #[test]
    fn forward_offset_leaves_an_offset_past_a_length_preserving_edit() {
        let (text, map) = apply_edits_mapped(
            "abc",
            vec![Edit::range_replacement("X".to_owned(), range(0, 1))],
        )
        .expect("woven");

        assert_eq!(text, "Xbc");
        assert_eq!(
            forward_offset(TextSize::new(2), &map, false),
            TextSize::new(2)
        );
    }

    #[test]
    fn forward_offset_slides_a_boundary_back_over_a_deletion() {
        let (text, map) =
            apply_edits_mapped("abcdef", vec![Edit::range_deletion(range(1, 3))]).expect("woven");

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
    fn forward_offset_slides_a_boundary_past_an_insertion() {
        let (text, map) = apply_edits_mapped(
            "abcdef",
            vec![Edit::insertion("XX".to_owned(), 2u32.into())],
        )
        .expect("woven");

        assert_eq!(text, "abXXcdef");
        assert_eq!(
            forward_offset(TextSize::new(1), &map, false),
            TextSize::new(1)
        );
        assert_eq!(
            forward_offset(TextSize::new(4), &map, false),
            TextSize::new(6)
        );
    }

    #[test]
    fn forward_offset_slides_the_final_boundary_past_an_end_insertion() {
        let (text, map) =
            apply_edits_mapped("abc", vec![Edit::insertion("XX".to_owned(), 3u32.into())])
                .expect("woven");

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
        let (text, map) =
            apply_edits_mapped(source.text(), vec![Edit::range_deletion(range(1, 11))])
                .expect("woven");
        let limit = text.text_len();

        let forwarded = forward_offsets(source.cell_offsets(), &map, limit);

        assert!(forwarded.iter().all(|offset| *offset <= limit));
        assert!(forwarded.windows(2).all(|pair| pair[0] <= pair[1]));
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

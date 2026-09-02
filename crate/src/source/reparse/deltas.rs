//! The old-to-new offset slide a rule's applied edits describe.

use itertools::Itertools;
use ruff_diagnostics::{SourceMap, SourceMarker};
use ruff_text_size::{TextRange, TextSize};

use crate::primitives::edit::shifted_past;

/// The old-to-new offset slide a rule's applied edits describe, read off
/// the `SourceMap` the weave of those edits produced.
pub(super) struct Deltas<'map> {
    markers: &'map [SourceMarker],
}

impl<'map> Deltas<'map> {
    pub(super) fn new(map: &'map SourceMap) -> Self {
        Self {
            markers: map.markers(),
        }
    }

    /// `offset` moved by the delta of the last marker at or before it,
    /// left where it is when no marker precedes it.
    pub(super) fn shift(&self, offset: TextSize) -> TextSize {
        shifted_past(offset, self.markers)
    }

    /// One span per marker pair, both endpoints read through `side`,
    /// which picks the buffer the spans are measured against.
    fn spans<F: Fn(&SourceMarker) -> TextSize>(
        &self,
        side: F,
    ) -> impl Iterator<Item = TextRange> + use<'_, 'map, F> {
        self.markers
            .iter()
            .tuples()
            .map(move |(start, end)| TextRange::new(side(start), side(end)))
    }

    /// True where nothing in `range` moves, the span closing before the
    /// first edit or the map carrying no edit at all.
    pub(super) fn holds_still(&self, range: TextRange) -> bool {
        self.markers
            .first()
            .is_none_or(|first| range.end() < first.source())
    }

    /// The spans the edits replaced, ascending, in the buffer they were
    /// measured against, one per marker pair.
    pub(super) fn replaced(&self) -> impl Iterator<Item = TextRange> + use<'_, 'map> {
        self.spans(SourceMarker::source)
    }

    /// `range` moved to where the woven text holds it.
    pub(super) fn slide(&self, range: TextRange) -> TextRange {
        TextRange::new(self.shift(range.start()), self.shift(range.end()))
    }

    /// `offset` moved by the delta of the last marker strictly before
    /// it, so an edit opening at `offset` lands past it rather than
    /// ahead of it.
    pub(super) fn shift_before(&self, offset: TextSize) -> TextSize {
        let ahead = self
            .markers
            .partition_point(|marker| marker.source() < offset);
        self.markers[..ahead]
            .last()
            .map_or(offset, |marker| marker.dest() + (offset - marker.source()))
    }

    /// A window over `held` moved to where the woven text holds it, an
    /// insertion at the window's start landing inside it rather than
    /// ahead of it, so the text the edits wrote there is the window's
    /// to reparse.
    pub(super) fn slide_window(&self, held: TextRange) -> TextRange {
        TextRange::new(self.shift_before(held.start()), self.shift(held.end()))
    }

    /// A token over `range` the reparse leaves standing, moved to where
    /// the woven text holds it, an insertion at a token's end landing
    /// past it and a zero-width token sitting past text inserted where
    /// it stands.
    pub(super) fn slide_token(&self, range: TextRange) -> TextRange {
        if range.is_empty() {
            self.slide(range)
        } else {
            TextRange::new(self.shift(range.start()), self.shift_before(range.end()))
        }
    }

    /// The spans the edits wrote, ascending, in the woven text.
    pub(super) fn written(&self) -> impl Iterator<Item = TextRange> + use<'_, 'map> {
        self.spans(SourceMarker::dest)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Edit;

    use super::*;
    use crate::testing::{range, replacement, woven};

    /// The map `edits` produce over `text`, the pairing [`Deltas`] reads.
    fn mapped(text: &str, edits: Vec<Edit>) -> SourceMap {
        woven(text, edits).1
    }

    #[rstest]
    #[case::a_span_closing_before_the_edit(range(0, 2), true)]
    #[case::a_span_closing_where_the_edit_opens(range(0, 3), false)]
    #[case::a_span_reaching_past_the_edit(range(0, 4), false)]
    #[case::a_span_opening_after_the_edit(range(5, 6), false)]
    fn holds_still_reads_whether_a_span_closes_ahead_of_the_first_edit(
        #[case] span: TextRange,
        #[case] expected: bool,
    ) {
        let map = mapped("abcdef", vec![replacement("X", 3, 4)]);

        assert_eq!(Deltas::new(&map).holds_still(span), expected);
    }

    #[test]
    fn replaced_and_written_pair_each_edit_with_its_woven_span() {
        let map = mapped("abcdef", vec![replacement("XX", 1, 2)]);
        let deltas = Deltas::new(&map);

        assert_eq!(deltas.replaced().collect::<Vec<_>>(), [range(1, 2)]);
        assert_eq!(deltas.written().collect::<Vec<_>>(), [range(1, 3)]);
    }

    #[rstest]
    #[case::ahead_of_every_edit(vec![replacement("XX", 3, 4)], range(0, 2), range(0, 2))]
    #[case::past_a_narrowing_edit(vec![Edit::range_deletion(range(1, 3))], range(4, 6), range(2, 4))]
    #[case::past_a_widening_edit(vec![replacement("XX", 1, 2)], range(3, 5), range(4, 6))]
    #[case::past_two_insertions(
        vec![
            Edit::insertion("X".to_owned(), 1u32.into()),
            Edit::insertion("Y".to_owned(), 3u32.into()),
        ],
        range(4, 6),
        range(6, 8)
    )]
    #[case::a_zero_width_range(vec![replacement("XX", 1, 2)], range(4, 4), range(5, 5))]
    fn slide_moves_a_range_by_the_deltas_ahead_of_it(
        #[case] edits: Vec<Edit>,
        #[case] span: TextRange,
        #[case] expected: TextRange,
    ) {
        let map = mapped("abcdef", edits);

        assert_eq!(Deltas::new(&map).slide(span), expected);
    }
}

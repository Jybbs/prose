//! The old-to-new offset slide a rule's applied edits describe.

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
    fn shift(&self, offset: TextSize) -> TextSize {
        shifted_past(offset, self.markers)
    }

    /// One span per marker pair, both endpoints read through `side`,
    /// which picks the buffer the spans are measured against.
    fn spans<F: Fn(&SourceMarker) -> TextSize>(
        &self,
        side: F,
    ) -> impl Iterator<Item = TextRange> + use<'_, 'map, F> {
        self.markers
            .chunks_exact(2)
            .map(move |pair| TextRange::new(side(&pair[0]), side(&pair[1])))
    }

    /// True where nothing in `range` moves, the span closing before the
    /// first edit or the map carrying no edit at all.
    pub(super) fn holds_still(&self, range: TextRange) -> bool {
        self.markers
            .first()
            .is_none_or(|first| range.end() <= first.source())
    }

    /// The spans the edits replaced, ascending, in the buffer they were
    /// measured against, one per marker pair.
    pub(super) fn replaced(&self) -> impl Iterator<Item = TextRange> + use<'_, 'map> {
        self.spans(SourceMarker::source)
    }

    /// `range` moved to where the woven text holds it.
    pub(super) fn slide(&self, range: TextRange) -> TextRange {
        let start = self.shift(range.start());
        if range.is_empty() {
            return TextRange::empty(start);
        }
        TextRange::new(start, self.shift(range.end()))
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
    #[case::a_span_closing_where_the_edit_opens(range(0, 3), true)]
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

    #[test]
    fn slide_carries_a_zero_width_range_to_one_offset() {
        let map = mapped("abcdef", vec![replacement("XX", 1, 2)]);

        let slid = Deltas::new(&map).slide(TextRange::empty(TextSize::new(4)));

        assert!(slid.is_empty());
        assert_eq!(slid.start(), TextSize::new(5));
    }

    #[test]
    fn slide_holds_a_range_ahead_of_every_edit() {
        let map = mapped("abcdef", vec![replacement("XX", 3, 4)]);

        assert_eq!(Deltas::new(&map).slide(range(0, 2)), range(0, 2));
    }

    #[test]
    fn slide_moves_a_range_past_a_narrowing_edit() {
        let map = mapped("abcdef", vec![Edit::range_deletion(range(1, 3))]);

        assert_eq!(Deltas::new(&map).slide(range(4, 6)), range(2, 4));
    }

    #[test]
    fn slide_moves_a_range_past_a_widening_edit() {
        let map = mapped("abcdef", vec![replacement("XX", 1, 2)]);

        assert_eq!(Deltas::new(&map).slide(range(3, 5)), range(4, 6));
    }

    #[test]
    fn slide_sums_the_deltas_of_every_edit_ahead_of_a_range() {
        let map = mapped(
            "abcdef",
            vec![
                Edit::insertion("X".to_owned(), 1u32.into()),
                Edit::insertion("Y".to_owned(), 3u32.into()),
            ],
        );

        assert_eq!(Deltas::new(&map).slide(range(4, 6)), range(6, 8));
    }
}

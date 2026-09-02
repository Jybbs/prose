//! Where a reflow lands: whether a fold fits the budget, whether the
//! row a pair sits on overflows it, the column a pair reaches once the
//! pass's earlier edits apply, and the spans an in-place shed removes.

use ruff_diagnostics::Edit;
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use super::{
    Shedder,
    plan::{Candidate, shed_columns},
};
use crate::primitives::{
    edit::{apply_inline_edits, insert_edit},
    inline::{display_width, end_column, run_closes_to_a_space, soft_wrap_runs},
    splice::splice_preserves_tree,
};

impl Shedder<'_> {
    /// The column `offset` reaches once the edits emitted so far apply,
    /// measured from the enclosing logical line.
    fn column_at(&self, offset: TextSize) -> usize {
        self.column_through(self.source.logical_line_start(offset))
    }

    /// The column the text through `range` reaches once the edits
    /// emitted so far apply, `range` opening at the row or the logical
    /// line the measure reads from.
    fn column_through(&self, range: TextRange) -> usize {
        end_column(&apply_inline_edits(self.source, range, &self.edits), 0)
    }

    /// The columns `candidate`'s joined interior takes, widened by the
    /// spaces its own flush sides keep and narrowed by the columns each
    /// nested candidate sheds alongside it. `None` for an interior no
    /// fold joins.
    fn joined_width(&self, candidate: &Candidate, candidates: &[Candidate]) -> Option<usize> {
        let bare = candidate.bare.as_ref()?;
        Some(
            display_width(bare) + candidate.flush.spaces()
                - shed_columns(candidate.inner, candidates),
        )
    }

    /// The column `offset` reaches once the edits emitted so far apply
    /// and `reflow-calls` takes its turn on the row: each outermost
    /// call ending ahead of `offset` on that row explodes while the row
    /// through `offset` still overflows the budget, dropping its closer
    /// to the row's indent and the text after it along with it.
    fn shifted_column(&self, offset: TextSize) -> usize {
        let column = self.column_at(offset);
        if column < self.code_line_length {
            return column;
        }
        let row_start = self.source.text().line_start(offset);
        let placed = |to: TextSize| self.column_through(TextRange::new(row_start, to));
        let indent = self.source.line_indent_width(offset);
        let mut shift = 0;
        let row = placed(offset);
        for call in self
            .calls
            .iter()
            .filter(|call| row_start <= call.start() && call.end() <= offset)
        {
            if row.saturating_sub(shift) < self.code_line_length {
                break;
            }
            shift = placed(call.end()).saturating_sub(indent + 1);
        }
        column.saturating_sub(shift)
    }

    /// The columns `pair`'s own row carries past its closing paren,
    /// narrowed by the parentheses this pass sheds along that stretch.
    fn tail_width(&self, pair: TextRange, candidates: &[Candidate]) -> usize {
        let tail = self.source.row_tail(pair.end());
        display_width(self.source.slice(tail)).saturating_sub(shed_columns(tail, candidates))
    }

    /// True when joining `candidate` leaves its line inside the budget
    /// once `reflow-calls` has taken its turn on the row, and false for
    /// an interior no fold joins.
    pub(super) fn fits(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let Some(width) = self.joined_width(candidate, candidates) else {
            return false;
        };
        self.shifted_column(candidate.pair.start()) + width <= self.code_line_length
    }

    /// True where joining `candidate` crosses the budget on the row the
    /// source puts it on, the measure a break answers. An interior no fold
    /// joins overflows outright.
    pub(super) fn overflows(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let Some(width) = self.joined_width(candidate, candidates) else {
            return true;
        };
        let row = self.column_at(candidate.pair.start())
            + width
            + self.tail_width(candidate.pair, candidates);
        row > self.code_line_length
    }

    /// Emits an edit closing each line-spanning whitespace run inside
    /// `inner`, to a single space between two tokens and to nothing
    /// against a bracket.
    pub(super) fn push_fold_edits(&mut self, inner: TextRange) {
        let text = self.source.slice(inner);
        for (begin, len) in soft_wrap_runs(text) {
            let start = inner.start() + TextSize::try_from(begin).expect("offset fits u32");
            let end = start + TextSize::try_from(len).expect("run length fits u32");
            let span = TextRange::new(start, end);
            insert_edit(
                &mut self.edits,
                if run_closes_to_a_space(text, begin, len) {
                    Edit::range_replacement(" ".to_owned(), span)
                } else {
                    Edit::range_deletion(span)
                },
            );
        }
    }

    /// The deletion spans shedding `candidate` in place, leaving its
    /// breaks where the source wrote them: the opening paren with the
    /// horizontal whitespace around it up to a break on either side, and
    /// the span from the interior's end through the closing paren. `None`
    /// where the splice does not preserve the statement tree, the shape a
    /// pair outside any enclosing bracket takes once its boundary break
    /// loses the paren that licensed it.
    pub(super) fn shed_in_place_spans(
        &self,
        candidate: &Candidate,
    ) -> Option<(TextRange, TextRange)> {
        let Candidate { inner, pair, .. } = *candidate;
        let text = self.source.text();
        let after = &text[pair.start().to_usize() + 1..];
        let trailing = after.text_len() - after.trim_whitespace_start().text_len();
        let mut open = TextRange::at(pair.start(), TextSize::of('(') + trailing);
        // The paren gone, whitespace ahead of it would trail its row, so
        // a break directly past the span pulls that run into the span.
        if text[open.end().to_usize()..].starts_with(['\r', '\n']) {
            let before = &text[..pair.start().to_usize()];
            let leading = before.text_len() - before.trim_whitespace_end().text_len();
            open = TextRange::new(open.start() - leading, open.end());
        }
        let close = TextRange::new(inner.end(), pair.end());
        let bare = self.source.slice(TextRange::new(open.end(), close.start()));
        splice_preserves_tree(self.source, pair, &candidate.flush.padded(bare))
            .then_some((open, close))
    }
}

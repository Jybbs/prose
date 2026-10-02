//! Where a reflow lands: whether a fold fits the budget, whether the
//! row a pair sits on overflows it, the column a pair reaches once the
//! pass's earlier edits apply and the padding and alignment rules settle
//! its row, and the spans an in-place shed removes.

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
    inline::{
        display_width, run_closes_to_a_space, settled_head_column, settled_width, soft_wrap_runs,
    },
    slots::starting_within,
    splice::splice_preserves_tree,
};

impl Shedder<'_> {
    /// True where `candidate`'s joined row crosses the budget and a break
    /// owns its shape, read for a pair later on the row of the candidate
    /// under test.
    fn breaks(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        if let Some(&breaks) = self.breaks.borrow().get(&candidate.pair) {
            return breaks;
        }
        let breaks = self.overflows(candidate, candidates)
            && self.broken_form(candidate, candidates).is_some();
        self.breaks.borrow_mut().insert(candidate.pair, breaks);
        breaks
    }

    /// The column `offset` reaches once the edits emitted so far apply
    /// and the padding and alignment rules settle its row, measured from
    /// the enclosing logical line.
    fn column_at(&self, offset: TextSize) -> usize {
        self.column_through(self.source.logical_line_start(offset))
    }

    /// The column the text through `range` reaches once the edits
    /// emitted so far apply and `strip-stranded-padding` settles the row
    /// it closes on, `range` opening at the row or the logical line the
    /// measure reads from. Text those edits leave on the row the source
    /// writes it on starts from the column `align-equals` shifts that row
    /// to.
    fn column_through(&self, range: TextRange) -> usize {
        let placed = apply_inline_edits(self.source, range, &self.edits);
        let column = settled_head_column(self.source, self.padding, &placed, range.end(), 0);
        self.reservations
            .column_under(self.source, &self.edits, range.end(), column)
    }

    /// The columns `candidate`'s joined interior takes, widened by the
    /// spaces its own flush sides keep and narrowed by the padding
    /// `strip-stranded-padding` drops inside it and the columns each
    /// nested candidate sheds alongside it. `None` for an interior no
    /// fold joins.
    fn joined_width(&self, candidate: &Candidate, candidates: &[Candidate]) -> Option<usize> {
        let bare = candidate.bare.as_ref()?;
        Some(
            settled_width(
                self.source,
                self.padding,
                candidate.inner,
                display_width(bare) + candidate.flush.spaces(),
            ) - shed_columns(candidate.inner, candidates),
        )
    }

    /// The column `offset` reaches once the edits emitted so far apply
    /// and `reflow-calls` takes its turn on the row. Each outermost
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

    /// The columns `pair`'s own row carries past its closing paren once
    /// `strip-stranded-padding` settles it, narrowed by the parentheses
    /// this pass sheds along that stretch. The measure closes at the
    /// opening paren of the first later pair on the row that breaks.
    fn tail_width(&self, pair: TextRange, candidates: &[Candidate]) -> usize {
        let mut tail = self.source.row_tail(pair.end());
        if let Some(later) = starting_within(candidates, tail, |other| other.pair.start())
            .find(|later| self.breaks(later, candidates))
        {
            tail = TextRange::new(tail.start(), later.pair.start() + TextSize::of('('));
        }
        settled_width(
            self.source,
            self.padding,
            tail,
            self.source.tail_width(tail),
        )
        .saturating_sub(shed_columns(tail, candidates))
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

    /// True where joining `candidate` crosses the budget on its row from
    /// the column [`Self::fits`] reads, the measure a break answers. An
    /// interior no fold joins overflows outright.
    pub(super) fn overflows(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let Some(width) = self.joined_width(candidate, candidates) else {
            return true;
        };
        let row = self.shifted_column(candidate.pair.start())
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
            let start = inner.start() + text[..begin].text_len();
            let end = start + text[begin..begin + len].text_len();
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

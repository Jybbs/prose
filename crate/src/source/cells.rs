//! Notebook cell boundaries, the offsets a notebook's cells open at
//! and the cuts that keep each boundary on a statement edge across a
//! reparse.

use ruff_notebook::{CellOffsets, Notebook};
use ruff_python_ast::{PySourceType, Stmt};
use ruff_python_parser::ParseError;
use ruff_source_file::{LineRanges, OneIndexed};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::Source;
use crate::primitives::slots::item_holding;

impl Source {
    /// Builds the concatenated source of a parsed notebook, attaching its
    /// cell boundaries and each code cell's notebook position. The caller
    /// keeps `notebook` to re-emit the document after formatting.
    pub(crate) fn from_notebook(
        notebook: &Notebook,
        name: impl Into<Box<str>>,
    ) -> Result<Self, ParseError> {
        let mut source = Self::build(
            notebook.source_code().to_owned(),
            name,
            PySourceType::Ipynb,
            notebook.cell_offsets().clone(),
        )?;
        source.cell_numbers = notebook
            .index()
            .iter()
            .map(|cell| cell.cell_index())
            .collect();
        Ok(source)
    }

    /// Returns the absolute notebook position of the code cell at
    /// `index`, counting Markdown cells, or `index` one-indexed for an
    /// ordinary module.
    pub(crate) fn cell_number(&self, index: usize) -> OneIndexed {
        self.cell_numbers
            .get(index)
            .copied()
            .unwrap_or_else(|| OneIndexed::from_zero_indexed(index))
    }

    /// Returns the notebook cell boundaries in the concatenated buffer,
    /// empty for an ordinary module.
    pub(crate) fn cell_offsets(&self) -> &CellOffsets {
        &self.cell_offsets
    }

    /// Returns `true` when the cell boundary at `index` sits on a
    /// statement boundary, meaning at a line start with no statement of
    /// the module body spanning it. An index past the last boundary
    /// qualifies, as does every boundary of an ordinary module.
    pub(crate) fn cell_splits_cleanly(&self, index: usize) -> bool {
        self.cell_offsets
            .get(index)
            .is_none_or(|&offset| splits_statements(offset, &self.ast().body, self.text()))
    }

    /// Returns the start of the notebook cell containing `offset`, or
    /// `None` for an ordinary module or an offset past the last cell.
    pub(crate) fn cell_start(&self, offset: TextSize) -> Option<TextSize> {
        self.cell_offsets
            .containing_range(offset)
            .map(TextRange::start)
    }

    /// Returns the source text of each notebook cell, the whole buffer
    /// as one slice for an ordinary module.
    pub fn cell_texts(&self) -> Vec<&str> {
        if !self.is_notebook() {
            return vec![self.text()];
        }
        self.cell_offsets
            .content_ranges()
            .map(|range| self.slice(range))
            .collect()
    }

    /// The full lines `range` spans, held back from the synthetic
    /// newline closing the notebook cell that holds it. An ordinary
    /// module takes the span unclamped, and a deletion over the result
    /// empties a cell rather than merging it into the next.
    pub(crate) fn full_lines_within_cell(&self, range: TextRange) -> TextRange {
        let lines = self.text().full_lines_range(range);
        let Some(cell) = self.cell_offsets.containing_range(range.start()) else {
            return lines;
        };
        let content_end = cell.end() - TextSize::of('\n');
        TextRange::new(lines.start(), lines.end().min(content_end))
    }

    /// Returns `true` when this source is a notebook, carrying at least
    /// one cell boundary. Always `false` for an ordinary module.
    pub(crate) fn is_notebook(&self) -> bool {
        !self.cell_offsets.is_empty()
    }

    /// Recuts `offsets` onto the statement boundaries `body` and `text`
    /// carry, moving a boundary that splits this source's statements but
    /// none of the replacement's to the start of the statement it now
    /// falls inside. One already run through a statement holds, as does
    /// one whose recut would not clear its predecessor, leaving `offsets`
    /// strictly ascending.
    pub(super) fn recut_cells(
        &self,
        mut offsets: CellOffsets,
        body: &[Stmt],
        text: &str,
    ) -> CellOffsets {
        for index in 1..offsets.len().saturating_sub(1) {
            let offset = offsets[index];
            if splits_statements(offset, body, text) || !self.cell_splits_cleanly(index) {
                continue;
            }
            let line_start = text.line_start(offset);
            let cut = statement_spanning(line_start, body).map_or(line_start, Ranged::start);
            if cut > offsets[index - 1] {
                offsets[index] = cut;
            }
        }
        offsets
    }

    /// Returns `true` when `a` and `b` sit in one notebook cell, with `a`
    /// at or before `b`. Always `true` for an ordinary module, which
    /// carries no cell boundary.
    pub(crate) fn same_cell(&self, a: TextSize, b: TextSize) -> bool {
        !self.cell_offsets.has_cell_boundary(TextRange::new(a, b))
    }
}

/// Returns `true` when `offset` opens a statement boundary in `text`,
/// sitting at a line start with no statement of `body` spanning it.
fn splits_statements(offset: TextSize, body: &[Stmt], text: &str) -> bool {
    text.is_at_start_of_line(offset) && statement_spanning(offset, body).is_none()
}

/// Returns the statement of `body` that `offset` falls strictly inside,
/// or `None` when `offset` sits at a statement's own start or between two.
fn statement_spanning(offset: TextSize, body: &[Stmt]) -> Option<&Stmt> {
    item_holding(body, offset).filter(|stmt| stmt.start() < offset && offset < stmt.end())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;
    use ruff_text_size::TextLen;

    use super::*;
    use crate::testing::{notebook, parse};

    /// Replaces `before`'s interior boundaries with `drifts` in order and
    /// its closing offset with `after`'s length, the shape a rule's edits
    /// leave behind when they slide a boundary off its statement.
    fn drifted_offsets(before: &Source, after: &Source, drifts: &[u32]) -> CellOffsets {
        let mut offsets = before.cell_offsets().clone();
        for (slot, &drift) in drifts.iter().enumerate() {
            offsets[slot + 1] = TextSize::new(drift);
        }
        *offsets
            .last_mut()
            .expect("a notebook carries a closing offset") = after.text().text_len();
        offsets
    }

    #[test]
    fn cell_number_counts_markdown_cells_and_survives_a_reparse() {
        let json = r##"{
            "cells": [
                {"cell_type": "markdown", "metadata": {}, "source": "# Notes"},
                {"cell_type": "code", "execution_count": null, "metadata": {},
                 "outputs": [], "source": "x = 1\n"}
            ],
            "metadata": {"language_info": {"name": "python"}},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##;
        let parsed = Notebook::from_source_code(json).expect("notebook parses");
        let nb = Source::from_notebook(&parsed, "<nb>").expect("notebook source builds");

        assert_eq!(nb.cell_number(0), OneIndexed::from_zero_indexed(1));

        let reparsed = nb
            .reparse_carrying(nb.text().to_owned(), nb.cell_offsets().clone())
            .expect("reparses");
        assert_eq!(reparsed.cell_number(0), OneIndexed::from_zero_indexed(1));
    }

    #[test]
    fn cell_number_falls_back_to_the_position_for_a_module() {
        assert_eq!(
            parse("x = 1\n").cell_number(3),
            OneIndexed::from_zero_indexed(3)
        );
    }

    #[test]
    fn cell_offsets_empty_for_a_module_and_present_for_a_notebook() {
        let module = Source::from_str("x = 1\n").expect("parses");
        assert!(module.cell_offsets().is_empty());

        let nb = notebook(&["x = 1\n", "y = 2\n"]);
        assert_eq!(nb.cell_offsets().first(), Some(&TextSize::new(0)));
        assert!(
            nb.cell_offsets().len() >= 2,
            "two cells open at least two boundaries",
        );
    }

    #[test]
    fn cell_splits_cleanly_breaks_where_a_cell_opens_inside_a_statement() {
        let nb = notebook(&["def helper():", "    return 1\n"]);
        assert!(nb.cell_splits_cleanly(0));
        assert!(!nb.cell_splits_cleanly(1));
    }

    #[test]
    fn cell_splits_cleanly_holds_at_every_boundary_of_whole_statements() {
        let nb = notebook(&["x = 1\n", "y = 2\n"]);
        assert!((0..nb.cell_offsets().len()).all(|index| nb.cell_splits_cleanly(index)));
    }

    #[test]
    fn cell_splits_cleanly_holds_past_the_last_boundary_and_for_a_module() {
        let nb = notebook(&["x = 1\n", "y = 2\n"]);
        assert!(nb.cell_splits_cleanly(nb.cell_offsets().len()));
        assert!(parse("x = 1\n").cell_splits_cleanly(0));
    }

    #[test]
    fn cell_texts_returns_the_whole_buffer_for_a_module() {
        assert_eq!(parse("x = 1\ny = 2\n").cell_texts(), vec!["x = 1\ny = 2\n"]);
    }

    #[test]
    fn full_lines_within_cell_holds_the_separator_closing_a_cell() {
        // The first cell carries no newline of its own, so the one that
        // ends its line is the separator `ruff_notebook` synthesized.
        let source = notebook(&["import os", "value = 1\n"]);
        let first = source.ast().body[0].range();

        assert_eq!(
            &source.text()[source.full_lines_within_cell(first)],
            "import os",
            "the span stops before the newline separating the cells",
        );
    }

    #[test]
    fn full_lines_within_cell_takes_the_whole_lines_of_an_ordinary_module() {
        let source = parse("import os\nvalue = 1\n");
        let first = source.ast().body[0].range();

        assert_eq!(
            &source.text()[source.full_lines_within_cell(first)],
            "import os\n"
        );
    }

    #[test]
    fn recut_cells_holds_a_cut_that_would_not_clear_the_previous_boundary() {
        let before = notebook(&["x = 1\n", "y = 2\n", "z = 3\n"]);
        let after = parse("def helper():\n    a = 1\n    b = 2\n");
        let offsets = drifted_offsets(&before, &after, &[18, 28]);

        // Both boundaries land inside the one `def`, whose start sits at
        // the opening offset, so moving either would collapse a cell to
        // nothing and leave `content_ranges` without its separator.
        let recut = before.recut_cells(offsets, &after.ast().body, after.text());
        assert_eq!(recut[1], TextSize::new(18));
        assert_eq!(recut[2], TextSize::new(28));
        assert!(recut.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[rstest]
    #[case::holds_a_boundary_already_on_a_statement(
        &["x = 1\n", "y = 2\n"],
        "x = 1\ny = 2\n",
        6,
        6,
    )]
    #[case::holds_an_authored_mid_statement_split(
        &["def helper():", "    return 1\n"],
        "x = 1\ndef helper():\n    return 1\n",
        20,
        20,
    )]
    #[case::moves_onto_the_spanning_statement_start(
        &["x = 1\n", "y = 2\n"],
        "x = 1\ndef helper():\n    return 1\n",
        20,
        6,
    )]
    #[case::pulls_back_to_the_line_start(
        &["x = 1\n", "y = 2\n"],
        "x = 1\n\n# note\ny = 2\n",
        10,
        7,
    )]
    fn recut_cells_lands_a_drifted_boundary(
        #[case] cells: &[&str],
        #[case] replacement: &str,
        #[case] drift: u32,
        #[case] expected: u32,
    ) {
        let before = notebook(cells);
        let after = parse(replacement);
        let offsets = drifted_offsets(&before, &after, &[drift]);

        let recut = before.recut_cells(offsets, &after.ast().body, after.text());
        assert_eq!(recut[1], TextSize::new(expected));
    }

    #[test]
    fn recut_cells_leaves_an_ordinary_module_offsets_untouched() {
        let module = parse("x = 1\n");
        let recut = module.recut_cells(
            module.cell_offsets().clone(),
            &module.ast().body,
            module.text(),
        );
        assert!(recut.is_empty());
    }
}

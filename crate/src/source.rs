//! Source-text wrapper bundling parsed AST, token stream, and line index.

use std::{path::Path, str::FromStr};

use itertools::Itertools;
use ruff_notebook::{CellOffsets, Notebook, NotebookError};
use ruff_python_ast::{
    AnyNodeRef, ExprRef, ModModule, PySourceType, Stmt,
    token::{Token, TokenKind, Tokens},
};
use ruff_python_parser::{ParseError, ParseOptions, Parsed, parse};
use ruff_python_trivia::{
    BackwardsTokenizer, CommentRanges, SimpleToken, SimpleTokenKind, lines_before,
};
use ruff_source_file::{
    LineColumn, LineEnding, LineRanges, OneIndexed, PositionEncoding, SourceFile,
    SourceFileBuilder, SourceLocation, find_newline,
};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use thiserror::Error;
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{binding::BindingAnalysis, inline::indent_width, range::paren_aware_range},
    suppression::SuppressionMap,
};

/// Owned wrapper around a parsed Python source file.
///
/// Holds the source text, the parsed AST, the token stream, a lazy
/// line index, a `CommentRanges` index built during parsing, and a
/// `SuppressionMap` of `# prose: off` / `# prose: skip` spans (plus
/// the `# fmt:` and `# yapf:` aliases), `# prose: skip[<id>]` per-rule
/// spans and `# prose: ignore[<id>]` per-line directives, plus a
/// `BindingAnalysis` table of every name's writes and reads.
/// `source_type` is the parse mode, `cell_offsets` the notebook cell
/// boundaries, and `cell_numbers` each cell's notebook position, the
/// last two empty for an ordinary module.
#[derive(Debug)]
pub struct Source {
    binding_analysis: Box<BindingAnalysis>,
    cell_numbers: Box<[OneIndexed]>,
    cell_offsets: CellOffsets,
    comment_ranges: CommentRanges,
    file: SourceFile,
    parsed: Parsed<ModModule>,
    source_type: PySourceType,
    suppression: Box<SuppressionMap>,
}

impl Source {
    fn build(
        text: String,
        name: impl Into<Box<str>>,
        source_type: PySourceType,
        cell_offsets: CellOffsets,
    ) -> Result<Self, ParseError> {
        let parsed = parse_typed_module(&text, source_type)?;
        Ok(Self::from_parsed(
            text,
            name,
            source_type,
            cell_offsets,
            parsed,
        ))
    }

    /// Builds a source carrying no notebook cell boundaries.
    pub(crate) fn build_module(
        text: String,
        name: impl Into<Box<str>>,
        source_type: PySourceType,
    ) -> Result<Self, ParseError> {
        Self::build(text, name, source_type, CellOffsets::default())
    }

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

    /// Wraps an already-parsed module in its indexes, the shared tail of
    /// every constructor.
    fn from_parsed(
        text: String,
        name: impl Into<Box<str>>,
        source_type: PySourceType,
        cell_offsets: CellOffsets,
        parsed: Parsed<ModModule>,
    ) -> Self {
        let file = SourceFileBuilder::new(name, text).finish();
        let comment_ranges = CommentRanges::from(parsed.tokens());
        let first_code_offset = parsed.syntax().body.first().map(Ranged::start);
        let suppression = Box::new(SuppressionMap::from_comments(
            &file.to_source_code(),
            &comment_ranges,
            parsed.tokens(),
            first_code_offset,
            &cell_offsets,
        ));
        let binding_analysis = Box::new(BindingAnalysis::new(parsed.syntax()));
        Self {
            binding_analysis,
            cell_numbers: Box::default(),
            cell_offsets,
            comment_ranges,
            file,
            parsed,
            source_type,
            suppression,
        }
    }

    /// Reads a file from disk and parses it. An `.ipynb` routes through
    /// the notebook reader, parsing its concatenated code cells.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::Io` if the read fails, `SourceError::Notebook`
    /// if an `.ipynb` is not a valid notebook, and `SourceError::Parse`
    /// if the parsed source is not a valid module.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, SourceError> {
        let path = path.as_ref();
        let text = fs_err::read_to_string(path)?;
        let source_type = PySourceType::try_from_path(path).unwrap_or_default();
        let name = path.display().to_string();
        if source_type.is_ipynb() {
            let notebook = Notebook::from_source_code(&text)?;
            return Self::from_notebook(&notebook, name).map_err(Into::into);
        }
        Self::build_module(text, name, source_type).map_err(Into::into)
    }

    /// Recuts `offsets` onto the statement boundaries `body` and `text`
    /// carry, moving a boundary that splits this source's statements but
    /// none of the replacement's to the start of the statement it now
    /// falls inside. One already run through a statement holds, as does
    /// one whose recut would not clear its predecessor, leaving `offsets`
    /// strictly ascending.
    fn recut_cells(&self, mut offsets: CellOffsets, body: &[Stmt], text: &str) -> CellOffsets {
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

    pub fn ast(&self) -> &ModModule {
        self.parsed.syntax()
    }

    /// Returns the binding-analysis table built during parsing.
    pub fn binding_analysis(&self) -> &BindingAnalysis {
        &self.binding_analysis
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

    /// The full lines `range` spans, held back from the synthetic
    /// newline closing the notebook cell that holds it. An ordinary
    /// module takes the span unclamped, and a deletion over the result
    /// empties a cell rather than merging it into the next.
    pub(crate) fn full_lines_within_cell(&self, range: TextRange) -> TextRange {
        let lines = self.text().full_lines_range(range);
        let Some(cell) = self.cell_offsets.containing_range(range.start()) else {
            return lines;
        };
        let content_end = cell.end() - TextSize::from(1);
        TextRange::new(lines.start(), lines.end().min(content_end))
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

    /// Returns this source's text when it differs from `original`, or
    /// `None` when they match.
    pub fn changed_from(&self, original: &str) -> Option<&str> {
        (self.text() != original).then_some(self.text())
    }

    /// Returns the zero-indexed character column of `offset` on its line.
    pub fn column_of(&self, offset: TextSize) -> usize {
        self.line_column(offset).column.to_zero_indexed()
    }

    /// Returns `true` when content of display `width` beginning at
    /// `offset`'s column extends past `budget`.
    pub fn column_overflows(&self, offset: TextSize, width: usize, budget: usize) -> bool {
        self.column_of(offset) + width > budget
    }

    /// Returns the comment-range index built during parsing.
    pub fn comment_ranges(&self) -> &CommentRanges {
        &self.comment_ranges
    }

    /// Returns `true` when `next_start` sits on the source line directly
    /// after `prev_end`'s line. A trailing comment on `prev_end`'s line
    /// keeps the two consecutive, whereas a standalone comment line or a
    /// blank line pushes `next_start` two or more lines down and breaks
    /// adjacency.
    pub fn consecutive_lines(&self, prev_end: TextSize, next_start: TextSize) -> bool {
        self.line_index(next_start) == self.line_index(prev_end).saturating_add(1)
    }

    /// Returns `true` when the source text in `ranged` carries at
    /// least one line break.
    pub fn contains_line_break<R: Ranged>(&self, ranged: R) -> bool {
        self.file.source_text().contains_line_break(ranged.range())
    }

    /// Returns the start offset of the first token in `range` for
    /// which `predicate` is true. Callers that need the full `&Token`
    /// (kind, range, flags) should chain
    /// `tokens().in_range(range).iter().find(...)` directly.
    pub fn first_token_offset_in_range<F>(
        &self,
        range: TextRange,
        mut predicate: F,
    ) -> Option<TextSize>
    where
        F: FnMut(&Token) -> bool,
    {
        self.tokens()
            .in_range(range)
            .iter()
            .find(|&t| predicate(t))
            .map(Token::start)
    }

    /// Returns `true` when at least one blank line separates the
    /// source ahead of `offset` from the preceding non-whitespace.
    pub fn has_blank_line_before(&self, offset: TextSize) -> bool {
        lines_before(offset, self.text()) >= 2
    }

    /// Returns `true` when at least one comment lies within `ranged`.
    pub fn intersects_comment<R: Ranged>(&self, ranged: R) -> bool {
        self.comment_ranges.intersects(ranged.range())
    }

    /// Returns `true` when this source is a notebook, carrying at least
    /// one cell boundary. Always `false` for an ordinary module.
    pub(crate) fn is_notebook(&self) -> bool {
        !self.cell_offsets.is_empty()
    }

    /// Returns the line and column for a byte offset. Columns count
    /// UTF scalar values (characters), not bytes. Line and column are
    /// both `OneIndexed`.
    pub fn line_column(&self, offset: TextSize) -> LineColumn {
        self.file.to_source_code().line_column(offset)
    }

    /// Returns the character-width of the leading-whitespace prefix on
    /// the line containing `offset`. Tabs and form-feeds count as one
    /// character each.
    pub fn line_indent_width(&self, offset: TextSize) -> usize {
        indent_width(self.text().line_str(offset))
    }

    /// Returns the one-indexed line number for `offset`.
    pub fn line_index(&self, offset: TextSize) -> OneIndexed {
        self.file.to_source_code().line_index(offset)
    }

    /// Returns the range from `offset` to the end of its logical line,
    /// the start of the first `Newline` token past it or the module's
    /// own end. A break inside a bracketed construct carries
    /// `NonLogicalNewline` and leaves the logical line open.
    pub fn logical_line_tail(&self, offset: TextSize) -> TextRange {
        let module_end = self.module_range().end();
        let end = self
            .first_token_offset_in_range(TextRange::new(offset, module_end), |token| {
                token.kind() == TokenKind::Newline
            })
            .unwrap_or(module_end);
        TextRange::new(offset, end)
    }

    /// Returns the range spanning the entire source text.
    pub fn module_range(&self) -> TextRange {
        TextRange::up_to(self.text().text_len())
    }

    /// Returns the line-ending sequence used in this source, or
    /// `"\n"` when the source carries no line break.
    pub fn newline_str(&self) -> &'static str {
        find_newline(self.text())
            .map_or(LineEnding::Lf, |(_, ending)| ending)
            .as_str()
    }

    /// Returns `expr`'s range widened to the explicit parentheses
    /// recovered against `parent`, threading this source's token stream.
    pub(crate) fn paren_aware_range(&self, expr: ExprRef, parent: AnyNodeRef) -> TextRange {
        paren_aware_range(expr, parent, self.tokens())
    }

    /// Returns the first non-trivia token scanning backward from
    /// `offset`, or `None` when the scan finds none.
    pub(crate) fn prev_non_trivia_token(&self, offset: TextSize) -> Option<SimpleToken> {
        BackwardsTokenizer::up_to(offset, self.text(), self.comment_ranges())
            .skip_trivia()
            .next()
    }

    /// Returns the end offset of the token preceding `offset`, scanning
    /// backward over whitespace and comments.
    pub(crate) fn prev_token_end(&self, offset: TextSize) -> TextSize {
        self.prev_non_trivia_token(offset)
            .expect("invariant: a token precedes the scanned offset")
            .end()
    }

    /// Reparses with replacement source text, preserving the original
    /// name, and carrying `cell_offsets` forward through [`Self::recut_cells`]
    /// so a notebook keeps its cell boundaries across a rule. Diagnostic
    /// labels keep the original path or `<source>` placeholder.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if `text` is not a valid Python module.
    pub(crate) fn reparse_carrying(
        &self,
        text: String,
        cell_offsets: CellOffsets,
    ) -> Result<Self, ParseError> {
        let parsed = parse_typed_module(&text, self.source_type)?;
        let cell_offsets = self.recut_cells(cell_offsets, &parsed.syntax().body, &text);
        let mut next = Self::from_parsed(
            text,
            self.file.name(),
            self.source_type,
            cell_offsets,
            parsed,
        );
        next.cell_numbers = self.cell_numbers.clone();
        Ok(next)
    }

    /// Returns `true` when `a` and `b` sit in one notebook cell, with `a`
    /// at or before `b`. Always `true` for an ordinary module, which
    /// carries no cell boundary.
    pub(crate) fn same_cell(&self, a: TextSize, b: TextSize) -> bool {
        !self.cell_offsets.has_cell_boundary(TextRange::new(a, b))
    }

    /// Returns `true` when `a` and `b` sit on one physical source line,
    /// meaning no line break falls in the gap between them.
    pub fn same_line(&self, a: TextSize, b: TextSize) -> bool {
        !self.contains_line_break(TextRange::new(a, b))
    }

    /// Returns the byte slice spanned by anything `Ranged`.
    ///
    /// Accepts a raw `TextRange` or any AST node. The returned `&str`
    /// is guaranteed to fall on `char` boundaries.
    pub fn slice<R: Ranged>(&self, ranged: R) -> &str {
        self.file.slice(ranged.range())
    }

    /// Borrows the underlying `SourceFile`.
    pub fn source_file(&self) -> &SourceFile {
        &self.file
    }

    /// Returns the line and character offset for a byte offset, with the
    /// character offset counted in `encoding`'s units. Both line and
    /// character offset are `OneIndexed`. The editor protocol publishes
    /// positions in a negotiated encoding, where `line_column` only ever
    /// counts characters.
    pub fn source_location(&self, offset: TextSize, encoding: PositionEncoding) -> SourceLocation {
        self.file.to_source_code().source_location(offset, encoding)
    }

    /// Returns the suppression index built during parsing.
    pub(crate) fn suppression_map(&self) -> &SuppressionMap {
        &self.suppression
    }

    pub fn text(&self) -> &str {
        self.file.source_text()
    }

    /// Yields each adjacent token pair with the source range between
    /// them, the trivia the lexer skipped.
    pub(crate) fn token_gaps(&self) -> impl Iterator<Item = (&Token, &Token, TextRange)> {
        self.tokens()
            .iter()
            .tuple_windows()
            .map(|(token, next)| (token, next, TextRange::new(token.end(), next.start())))
    }

    /// Borrows the token stream produced during parsing.
    pub fn tokens(&self) -> &Tokens {
        self.parsed.tokens()
    }

    /// Returns the range of the trailing comma immediately before the
    /// closing bracket of `container`, or `None` when the last
    /// non-trivia token there is not a comma.
    pub(crate) fn trailing_comma(&self, container: TextRange) -> Option<TextRange> {
        self.prev_non_trivia_token(container.end() - TextSize::from(1u32))
            .filter(|token| token.kind() == SimpleTokenKind::Comma)
            .map(|token| token.range)
    }

    /// Returns the display width of the source text between `a` and `b`.
    pub(crate) fn width_between(&self, a: TextSize, b: TextSize) -> usize {
        self.slice(TextRange::new(a, b)).width()
    }
}

/// Parses Python source from an in-memory string.
///
/// The resulting `Source` carries the synthetic name `<source>` for
/// diagnostics.
impl FromStr for Source {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::build_module(text.to_owned(), "<source>", PySourceType::default())
    }
}

/// Failure to load and parse a source file from disk.
#[derive(Debug, Error)]
pub enum SourceError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Notebook(#[from] NotebookError),
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Parses `text` in `source_type`'s mode as a module.
fn parse_typed_module(
    text: &str,
    source_type: PySourceType,
) -> Result<Parsed<ModModule>, ParseError> {
    let Some(parsed) = parse(text, ParseOptions::from(source_type))?.try_into_module() else {
        unreachable!("module-mode parse never yields a bare expression");
    };
    Ok(parsed)
}

/// Returns `true` when `offset` opens a statement boundary in `text`,
/// sitting at a line start with no statement of `body` spanning it.
fn splits_statements(offset: TextSize, body: &[Stmt], text: &str) -> bool {
    text.is_at_start_of_line(offset) && statement_spanning(offset, body).is_none()
}

/// Returns the statement of `body` that `offset` falls strictly inside,
/// or `None` when `offset` sits at a statement's own start or between two.
fn statement_spanning(offset: TextSize, body: &[Stmt]) -> Option<&Stmt> {
    let after = body.partition_point(|stmt| stmt.start() < offset);
    body[..after].last().filter(|stmt| offset < stmt.end())
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;
    use ruff_python_ast::token::TokenKind;
    use ruff_source_file::OneIndexed;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::{
        primitives::scope::sub_bodies,
        testing::{assert_send_sync, notebook, parse, range},
    };

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

    fn line_column(line: usize, column: usize) -> LineColumn {
        LineColumn {
            line: OneIndexed::from_zero_indexed(line),
            column: OneIndexed::from_zero_indexed(column),
        }
    }

    #[test]
    fn build_with_ipynb_parses_a_line_magic() {
        let s = Source::build(
            "%matplotlib inline\nx = 1\n".to_owned(),
            "<nb>",
            PySourceType::Ipynb,
            CellOffsets::default(),
        )
        .expect("ipython mode parses a magic");
        assert_eq!(s.ast().body.len(), 2);
    }

    #[test]
    fn build_with_python_rejects_a_line_magic() {
        let result = Source::build(
            "%matplotlib inline\n".to_owned(),
            "<mod>",
            PySourceType::Python,
            CellOffsets::default(),
        );
        assert!(result.is_err());
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
    fn cell_texts_returns_the_whole_buffer_for_a_module() {
        assert_eq!(parse("x = 1\ny = 2\n").cell_texts(), vec!["x = 1\ny = 2\n"]);
    }

    #[test]
    fn changed_from_returns_none_when_text_matches() {
        let s = Source::from_str("x = 1\n").expect("parses");
        assert!(s.changed_from("x = 1\n").is_none());
    }

    #[test]
    fn changed_from_returns_text_when_it_differs() {
        let s = Source::from_str("x = 1\n").expect("parses");
        assert_eq!(s.changed_from("y = 2\n"), Some("x = 1\n"));
    }

    #[test]
    fn comment_ranges_indexes_each_comment_in_the_source() {
        let s = Source::from_str("# top\nx = 1  # trail\n").expect("parses");
        let ranges = s.comment_ranges();
        assert!(ranges.intersects(range(0, 1)));
        assert!(ranges.intersects(range(13, 14)));
    }

    #[rstest]
    #[case("a = 1\nb = 2\n", true)]
    #[case("a = 1  # trailing\nb = 2\n", true)]
    #[case("a = 1\n\nb = 2\n", false)]
    #[case("a = 1\n# standalone\nb = 2\n", false)]
    fn consecutive_lines_tolerates_trailing_comment_but_breaks_on_gap(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(
            source.consecutive_lines(body[0].end(), body[1].start()),
            expected,
        );
    }

    #[test]
    fn empty_input_parses_as_empty_module() {
        let s = Source::from_str("").expect("empty source parses");
        assert_eq!(s.text(), "");
        assert!(s.ast().body.is_empty());
    }

    #[test]
    fn first_token_offset_in_range_returns_first_match_when_multiple_satisfy() {
        // Chained assignment carries two `=` tokens, and the helper
        // must return the leftmost one, not just any match.
        let s = Source::from_str("a = b = 1\n").expect("parses");
        let offset = s
            .first_token_offset_in_range(s.ast().body[0].range(), |t| t.kind() == TokenKind::Equal)
            .expect("two `=` tokens, picks first");

        assert_eq!(offset, TextSize::new(2));
    }

    #[test]
    fn first_token_offset_in_range_returns_none_for_empty_range() {
        let s = Source::from_str("x = 1\n").expect("parses");
        let empty = TextRange::empty(TextSize::new(0));

        assert!(s.first_token_offset_in_range(empty, |_| true).is_none());
    }

    #[test]
    fn first_token_offset_in_range_returns_none_when_no_token_matches() {
        let s = Source::from_str("x = 1\n").expect("parses");
        let result = s
            .first_token_offset_in_range(s.ast().body[0].range(), |t| t.kind() == TokenKind::Colon);

        assert!(result.is_none());
    }

    #[test]
    fn first_token_offset_in_range_returns_offset_for_single_match() {
        let s = Source::from_str("x = 1\n").expect("parses");
        let offset = s
            .first_token_offset_in_range(s.ast().body[0].range(), |t| t.kind() == TokenKind::Equal)
            .expect("one `=` token");

        assert_eq!(offset, TextSize::new(2));
    }

    #[test]
    fn first_token_offset_in_range_supports_predicate_compositions() {
        // Mirrors how align_equals's aug-assign arm picks any token in
        // the augmented-assign-operator family rather than a specific kind.
        let s = Source::from_str("x += 1\n").expect("parses");
        let offset = s
            .first_token_offset_in_range(s.ast().body[0].range(), |t| {
                t.kind().as_augmented_assign_operator().is_some()
            })
            .expect("`+=` is an aug-assign operator");

        assert_eq!(offset, TextSize::new(2));
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
    fn from_path_bad_syntax_returns_parse_error() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file creates");
        std::fs::write(tmp.path(), b"def foo(").expect("temp file writes");

        let result = Source::from_path(tmp.path());
        assert_matches!(result, Err(SourceError::Parse(_)));
    }

    #[test]
    fn from_path_malformed_notebook_returns_notebook_error() {
        let tmp = tempfile::Builder::new()
            .suffix(".ipynb")
            .tempfile()
            .expect("temp file creates");
        std::fs::write(tmp.path(), b"{not valid json").expect("temp file writes");

        let result = Source::from_path(tmp.path());
        assert_matches!(result, Err(SourceError::Notebook(_)));
    }

    #[test]
    fn from_path_missing_file_returns_io_error() {
        let result = Source::from_path("/definitely/does/not/exist.py");
        assert_matches!(result, Err(SourceError::Io(_)));
    }

    #[test]
    fn from_path_reads_and_parses_an_existing_file() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file creates");
        std::fs::write(tmp.path(), b"x = 1\n").expect("temp file writes");

        let s = Source::from_path(tmp.path()).expect("existing file parses");
        assert_eq!(s.text(), "x = 1\n");
        assert_eq!(s.ast().body.len(), 1);
    }

    #[test]
    fn line_column_counts_characters_not_bytes() {
        let src = "αβγ";
        let s = Source::from_str(src).expect("multibyte source parses");
        assert_eq!(s.line_column(TextSize::new(6)), line_column(0, 3));
    }

    #[test]
    fn line_column_handles_unix_newlines() {
        let src = "a\nb\nc\n";
        let s = Source::from_str(src).expect("LF input parses");
        assert_eq!(s.line_column(TextSize::new(0)), line_column(0, 0));
        assert_eq!(s.line_column(TextSize::new(2)), line_column(1, 0));
        assert_eq!(s.line_column(TextSize::new(4)), line_column(2, 0));
    }

    #[test]
    fn line_column_handles_windows_newlines() {
        let src = "a\r\nb\r\nc\r\n";
        let s = Source::from_str(src).expect("CRLF input parses");
        assert_eq!(s.line_column(TextSize::new(0)), line_column(0, 0));
        assert_eq!(s.line_column(TextSize::new(3)), line_column(1, 0));
        assert_eq!(s.line_column(TextSize::new(6)), line_column(2, 0));
    }

    #[test]
    fn parse_error_returns_ruff_parse_error() {
        let result: Result<Source, ParseError> = Source::from_str("def foo(");
        assert!(result.is_err());
    }

    #[rstest]
    #[case("class C:\n    pass\n", "class C:")]
    #[case("class C:  # eol\n    pass\n", "class C:")]
    #[case("class C:\n    # comment\n    pass\n", "class C:")]
    #[case("def f():\n    pass\n", "def f():")]
    #[case("def f(\n    x,\n    y,\n):\n    pass\n", "def f(\n    x,\n    y,\n):")]
    fn prev_token_end_lands_past_the_header_colon(#[case] src: &str, #[case] header: &str) {
        let source = parse(src);
        let (body, _) = sub_bodies(&source.ast().body[0])[0];
        let end = source.prev_token_end(body[0].start());
        assert_eq!(&source.text()[..end.to_usize()], header);
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

    #[test]
    fn reparse_preserves_ipython_mode() {
        let s = Source::build(
            "%matplotlib inline\nx = 1\n".to_owned(),
            "<nb>",
            PySourceType::Ipynb,
            CellOffsets::default(),
        )
        .expect("parses");
        // Reparse carries the Ipython parse mode forward, so a magic in
        // the replacement source still parses as a node rather than a
        // syntax error.
        let reparsed = s
            .reparse_carrying(
                "%matplotlib inline\ny = 2\n".to_owned(),
                CellOffsets::default(),
            )
            .expect("reparses");
        assert_eq!(reparsed.ast().body.len(), 2);
    }

    #[test]
    fn reparse_returns_parse_error_for_bad_replacement() {
        let s = Source::from_str("x = 1\n").expect("original parses");
        let result = s.reparse_carrying("def foo(".to_owned(), CellOffsets::default());
        assert!(result.is_err());
    }

    #[rstest]
    #[case("a = 1; b = 2\n", true)]
    #[case("a = 1\nb = 2\n", false)]
    fn same_line_holds_within_a_line_and_breaks_across_one(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(source.same_line(body[0].end(), body[1].start()), expected);
    }

    #[test]
    fn single_character_input_parses() {
        let s = Source::from_str("x").expect("single name parses");
        assert_eq!(s.text(), "x");
        assert_eq!(s.ast().body.len(), 1);
    }

    #[test]
    fn slice_accepts_ast_nodes_via_ranged() {
        let s = Source::from_str("x = 1\n").expect("assignment parses");
        let stmt = s.ast().body.first().expect("one statement");
        assert_eq!(s.slice(stmt), "x = 1");
    }

    #[test]
    fn slice_at_multibyte_boundary_returns_full_codepoint() {
        let src = "α = 1";
        let s = Source::from_str(src).expect("multibyte source parses");
        let alpha = s.slice(range(0, 2));
        assert_eq!(alpha, "α");
    }

    #[test]
    fn source_is_send_and_sync() {
        assert_send_sync::<Source>();
    }

    #[test]
    fn tokens_returns_non_empty_stream_for_non_empty_source() {
        let s = Source::from_str("x = 1").expect("simple assignment parses");
        assert!(s.tokens().iter().next().is_some());
    }
}

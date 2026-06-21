//! Source-text wrapper bundling parsed AST, token stream, and line index.

use std::{path::Path, str::FromStr};

use ruff_python_ast::{
    AnyNodeRef, ExprRef, ModModule,
    token::{Token, Tokens},
};
use ruff_python_parser::{ParseError, Parsed, parse_module};
use ruff_python_trivia::{
    BackwardsTokenizer, CommentRanges, SimpleToken, SimpleTokenKind, leading_indentation,
    lines_before,
};
use ruff_source_file::{
    LineColumn, LineEnding, LineRanges, OneIndexed, PositionEncoding, SourceFile,
    SourceFileBuilder, SourceLocation, find_newline,
};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use thiserror::Error;

use crate::{
    primitives::{binding::BindingAnalysis, range::paren_aware_range},
    suppression::SuppressionMap,
};

/// Owned wrapper around a parsed Python source file.
///
/// Holds the source text, the parsed AST, the token stream, a lazy
/// line index, a `CommentRanges` index built during parsing, and a
/// `SuppressionMap` of `# prose: off` / `# prose: skip` spans (plus
/// the `# fmt:` and `# yapf:` aliases), `# prose: skip[<id>]` and
/// `# prose: ignore[<id>]` per-line directives, plus a
/// `BindingAnalysis` table of every name's writes and reads.
#[derive(Debug)]
pub struct Source {
    binding_analysis: Box<BindingAnalysis>,
    comment_ranges: CommentRanges,
    file: SourceFile,
    parsed: Parsed<ModModule>,
    suppression: Box<SuppressionMap>,
}

impl Source {
    pub(crate) fn build(text: String, name: impl Into<Box<str>>) -> Result<Self, ParseError> {
        let parsed = parse_module(&text)?;
        let file = SourceFileBuilder::new(name, text).finish();
        let comment_ranges = CommentRanges::from(parsed.tokens());
        let first_code_offset = parsed.syntax().body.first().map(Ranged::start);
        let suppression = Box::new(SuppressionMap::from_comments(
            &file.to_source_code(),
            &comment_ranges,
            first_code_offset,
        ));
        let binding_analysis = Box::new(BindingAnalysis::new(parsed.syntax()));
        Ok(Self {
            binding_analysis,
            comment_ranges,
            file,
            parsed,
            suppression,
        })
    }

    /// Reads a file from disk and parses it as Python source.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::Io` if the read fails and `SourceError::Parse`
    /// if the bytes are read successfully but do not form a valid module.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, SourceError> {
        let path = path.as_ref();
        let text = fs_err::read_to_string(path)?;
        Self::build(text, path.display().to_string()).map_err(Into::into)
    }

    pub fn ast(&self) -> &ModModule {
        self.parsed.syntax()
    }

    /// Returns the binding-analysis table built during parsing.
    pub fn binding_analysis(&self) -> &BindingAnalysis {
        &self.binding_analysis
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
    /// adjacency. Contrast [`Self::is_line_adjacent`], which breaks on any
    /// comment in the gap.
    pub fn consecutive_lines(&self, prev_end: TextSize, next_start: TextSize) -> bool {
        self.line_index(next_start) == self.line_index(prev_end).saturating_add(1)
    }

    /// Returns `true` when the source text in `ranged` carries at
    /// least one line break.
    pub fn contains_line_break<R: Ranged>(&self, ranged: R) -> bool {
        self.file.source_text().contains_line_break(ranged.range())
    }

    /// Returns the source name. For `from_path` inputs this is
    /// `path.display().to_string()`, for `from_str` inputs the
    /// synthetic placeholder `<source>`.
    pub fn filename(&self) -> &str {
        self.file.name()
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

    /// Returns `true` when the gap between two AST nodes carries
    /// exactly one newline and no comment, meaning the surrounding
    /// nodes sit on directly adjacent source lines. Contrast
    /// [`Self::consecutive_lines`], which rides a trailing comment on the
    /// preceding line.
    pub fn is_line_adjacent(&self, gap: TextRange) -> bool {
        !self.slice(gap).contains('#') && lines_before(gap.end(), self.text()) == 1
    }

    /// Returns the line and column for a byte offset. Columns count
    /// UTF scalar values (characters), not bytes. Line and column are
    /// both `OneIndexed`.
    pub fn line_column(&self, offset: TextSize) -> LineColumn {
        self.file.to_source_code().line_column(offset)
    }

    /// Returns the character-width of the leading-whitespace prefix on
    /// the line containing `offset`. Tabs and form-feeds count as one
    /// character each. Recognizes Python's full whitespace set via
    /// `ruff_python_trivia`.
    pub fn line_indent_width(&self, offset: TextSize) -> usize {
        leading_indentation(self.text().line_str(offset))
            .chars()
            .count()
    }

    /// Returns the one-indexed line number for `offset`.
    pub fn line_index(&self, offset: TextSize) -> OneIndexed {
        self.file.to_source_code().line_index(offset)
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

    /// Reparses with replacement source text, preserving the original name.
    ///
    /// Diagnostic labels keep the original path or `<source>` placeholder.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if `text` is not a valid Python module.
    pub fn reparse(&self, text: String) -> Result<Self, ParseError> {
        Self::build(text, self.file.name())
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
}

/// Parses Python source from an in-memory string.
///
/// The resulting `Source` carries the synthetic name `<source>` for
/// diagnostics.
impl FromStr for Source {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::build(text.to_owned(), "<source>")
    }
}

/// Failure to load and parse a Python source file from disk.
#[derive(Debug, Error)]
pub enum SourceError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] ParseError),
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use rstest::rstest;
    use ruff_python_ast::token::TokenKind;
    use ruff_source_file::OneIndexed;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::testing::{assert_send_sync, parse, range};

    fn line_column(line: usize, column: usize) -> LineColumn {
        LineColumn {
            line: OneIndexed::from_zero_indexed(line),
            column: OneIndexed::from_zero_indexed(column),
        }
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
    fn from_path_bad_syntax_returns_parse_error() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file creates");
        std::fs::write(tmp.path(), b"def foo(").expect("temp file writes");

        let result = Source::from_path(tmp.path());
        assert_matches!(result, Err(SourceError::Parse(_)));
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

    #[test]
    fn reparse_returns_parse_error_for_bad_replacement() {
        let s = Source::from_str("x = 1\n").expect("original parses");
        let result = s.reparse("def foo(".to_owned());
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

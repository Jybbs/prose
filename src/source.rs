//! Source-text wrapper bundling parsed AST, token stream, and line index.
//!
//! Every rule reads through `Source`. The text is owned rather than
//! borrowed, so `Source` carries no lifetime parameter and can move
//! across thread boundaries without lifetime gymnastics.

use std::path::Path;
use std::str::FromStr;

use ruff_python_ast::token::{Token, Tokens};
use ruff_python_ast::ModModule;
use ruff_python_parser::{parse_module, ParseError, Parsed};
use ruff_python_trivia::{leading_indentation, lines_before, CommentRanges};
use ruff_source_file::{LineColumn, LineRanges, SourceFile, SourceFileBuilder};
use ruff_text_size::{Ranged, TextRange, TextSize};
use thiserror::Error;

/// Owned wrapper around a parsed Python source file.
///
/// Holds the source text, the parsed AST, the token stream, a lazy
/// line index, and a `CommentRanges` index built during parsing.
#[derive(Debug)]
pub struct Source {
    comment_ranges: CommentRanges,
    file: SourceFile,
    parsed: Parsed<ModModule>,
}

impl Source {
    /// Reads a file from disk and parses it as Python source.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::Io` if the read fails and `SourceError::Parse`
    /// if the bytes are read successfully but do not form a valid module.
    /// The underlying `std::io::Error` from `fs_err` carries the path in
    /// its `Display`, so no additional wrapping is needed.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, SourceError> {
        let path = path.as_ref();
        let text = fs_err::read_to_string(path)?;
        Self::build(text, path.display().to_string()).map_err(Into::into)
    }

    fn build(text: String, name: impl Into<Box<str>>) -> Result<Self, ParseError> {
        let parsed = parse_module(&text)?;
        let file = SourceFileBuilder::new(name, text).finish();
        let comment_ranges = CommentRanges::from(parsed.tokens());
        Ok(Self {
            comment_ranges,
            file,
            parsed,
        })
    }

    pub fn ast(&self) -> &ModModule {
        self.parsed.syntax()
    }

    /// Returns the comment-range index built during parsing.
    pub fn comment_ranges(&self) -> &CommentRanges {
        &self.comment_ranges
    }

    /// Returns the zero-indexed character column of `offset` on its line.
    pub fn column_of(&self, offset: TextSize) -> usize {
        self.line_col(offset).column.to_zero_indexed()
    }

    /// Returns `true` when the source text in `ranged` carries at
    /// least one line break.
    pub fn contains_line_break<R: Ranged>(&self, ranged: R) -> bool {
        self.file.source_text().contains_line_break(ranged.range())
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

    /// Returns the start offset of the first token in `range` for
    /// which `predicate` is true. Callers that need the full `&Token`
    /// (kind, range, flags) should chain
    /// `tokens().in_range(range).iter().find(...)` directly. This
    /// helper exists for the dominant case of "find a kind, take its
    /// start offset," which every alignment-rule member constructor
    /// reduces to.
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

    /// Returns `true` when at least one comment lies within `ranged`.
    pub fn intersects_comment<R: Ranged>(&self, ranged: R) -> bool {
        self.comment_ranges.intersects(ranged.range())
    }

    /// Returns `true` when the gap between two AST nodes carries
    /// exactly one newline and no comment, meaning the surrounding
    /// nodes sit on directly adjacent source lines.
    pub fn is_line_adjacent(&self, gap: TextRange) -> bool {
        !self.slice(gap).contains('#') && lines_before(gap.end(), self.text()) == 1
    }

    /// Returns the line and column for a byte offset.
    ///
    /// Columns count UTF scalar values (characters), not bytes, so a
    /// multi-byte sequence advances the column by one rather than by its
    /// byte length. Line and column are both `OneIndexed`. Call
    /// `to_zero_indexed()` on either field when a zero-based index is needed.
    fn line_col(&self, offset: TextSize) -> LineColumn {
        self.file.to_source_code().line_column(offset)
    }

    /// Reparses with replacement source text, preserving the original name.
    ///
    /// The pipeline calls this after each rule applies its edit list, so the
    /// next rule sees a freshly-parsed AST whose ranges point into the new
    /// buffer. Diagnostic labels keep the original path or `<source>` placeholder.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if `text` is not a valid Python module.
    pub fn reparse(&self, text: String) -> Result<Self, ParseError> {
        Self::build(text, self.file.name())
    }

    /// Returns the byte slice spanned by anything `Ranged`.
    ///
    /// Accepts a raw `TextRange` or any AST node, since every node in
    /// `ruff_python_ast` implements `Ranged`. The returned `&str` is
    /// guaranteed to fall on `char` boundaries because the parser only
    /// emits ranges aligned to source token boundaries.
    pub fn slice<R: Ranged>(&self, ranged: R) -> &str {
        self.file.slice(ranged.range())
    }

    pub fn text(&self) -> &str {
        self.file.source_text()
    }

    /// Borrows the token stream produced during parsing.
    ///
    /// Rules that need whitespace-aware context (alignment, padding,
    /// trailing-comma stripping) re-lex through this stream because
    /// Ruff's AST is not whitespace-preserving.
    pub fn tokens(&self) -> &Tokens {
        self.parsed.tokens()
    }
}

/// Parses Python source from an in-memory string.
///
/// The resulting `Source` carries the synthetic name `<source>` for
/// diagnostics. Callers can invoke this through `Source::from_str(text)`
/// (with `std::str::FromStr` in scope) or via `text.parse::<Source>()`.
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
    use ruff_python_ast::token::TokenKind;
    use ruff_source_file::OneIndexed;
    use ruff_text_size::TextRange;

    use super::*;

    fn line_col(line: usize, column: usize) -> LineColumn {
        LineColumn {
            line: OneIndexed::from_zero_indexed(line),
            column: OneIndexed::from_zero_indexed(column),
        }
    }

    #[test]
    fn comment_ranges_indexes_each_comment_in_the_source() {
        let s = Source::from_str("# top\nx = 1  # trail\n").expect("parses");
        let ranges = s.comment_ranges();
        assert!(ranges.intersects(TextRange::new(TextSize::new(0), TextSize::new(1))));
        assert!(ranges.intersects(TextRange::new(TextSize::new(13), TextSize::new(14))));
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
        assert!(matches!(result, Err(SourceError::Parse(_))));
    }

    #[test]
    fn from_path_missing_file_returns_io_error() {
        let result = Source::from_path("/definitely/does/not/exist.py");
        assert!(matches!(result, Err(SourceError::Io(_))));
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
    fn line_col_counts_characters_not_bytes() {
        let src = "αβγ";
        let s = Source::from_str(src).expect("multibyte source parses");
        assert_eq!(s.line_col(TextSize::new(6)), line_col(0, 3));
    }

    #[test]
    fn line_col_handles_unix_newlines() {
        let src = "a\nb\nc\n";
        let s = Source::from_str(src).expect("LF input parses");
        assert_eq!(s.line_col(TextSize::new(0)), line_col(0, 0));
        assert_eq!(s.line_col(TextSize::new(2)), line_col(1, 0));
        assert_eq!(s.line_col(TextSize::new(4)), line_col(2, 0));
    }

    #[test]
    fn line_col_handles_windows_newlines() {
        let src = "a\r\nb\r\nc\r\n";
        let s = Source::from_str(src).expect("CRLF input parses");
        assert_eq!(s.line_col(TextSize::new(0)), line_col(0, 0));
        assert_eq!(s.line_col(TextSize::new(3)), line_col(1, 0));
        assert_eq!(s.line_col(TextSize::new(6)), line_col(2, 0));
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
        let alpha = s.slice(TextRange::new(TextSize::new(0), TextSize::new(2)));
        assert_eq!(alpha, "α");
    }

    #[test]
    fn source_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Source>();
    }

    #[test]
    fn tokens_returns_non_empty_stream_for_non_empty_source() {
        let s = Source::from_str("x = 1").expect("simple assignment parses");
        assert!(s.tokens().iter().next().is_some());
    }
}

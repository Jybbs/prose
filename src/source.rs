//! Source-text wrapper bundling parsed AST, token stream, and line index.
//!
//! Every rule reads through `Source`. The text is owned rather than
//! borrowed, so `Source` carries no lifetime parameter and can move
//! across thread boundaries without lifetime gymnastics.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ruff_python_ast::ModModule;
use ruff_python_ast::token::Tokens;
use ruff_python_parser::{Parsed, parse_module};
use ruff_source_file::{LineColumn, SourceFile, SourceFileBuilder};
use ruff_text_size::{Ranged, TextSize};
use thiserror::Error;


/// Failure to parse a buffer of Python source.
///
/// The wrapped `ruff_python_parser::ParseError` is exposed so callers
/// can inspect the error's location and category without going through
/// `Display`.
#[derive(Debug, Error)]
#[error("python parse failed: {0}")]
pub struct ParseError(#[from] pub ruff_python_parser::ParseError);


/// Owned wrapper around a parsed Python source file.
///
/// Holds the source text, the parsed AST, the token stream, and a lazy
/// line index.
pub struct Source {
    file   : SourceFile,
    parsed : Parsed<ModModule>,
}


impl Source {
    /// Reads a file from disk and parses it as Python source.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::Io` if the read fails and `SourceError::Parse`
    /// if the bytes are read successfully but do not form a valid module.
    pub fn from_path(path: &Path) -> Result<Self, SourceError> {
        let text = fs::read_to_string(path).map_err(|source| SourceError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::build(text, path.display().to_string()).map_err(SourceError::Parse)
    }

    fn build(text: String, name: String) -> Result<Self, ParseError> {
        let parsed = parse_module(&text)?;
        let file   = SourceFileBuilder::new(name, text).finish();
        Ok(Self { file, parsed })
    }

    pub fn ast(&self) -> &ModModule {
        self.parsed.syntax()
    }

    /// Converts a byte offset into a zero-indexed `(line, column)` pair.
    ///
    /// Columns count UTF scalar values (characters), not bytes, so a
    /// multi-byte sequence advances the column by one rather than by its
    /// byte length.
    pub fn line_col(&self, offset: TextSize) -> (usize, usize) {
        let LineColumn { line, column } = self.file.to_source_code().line_column(offset);
        (line.to_zero_indexed(), column.to_zero_indexed())
    }

    /// Borrows the source's name.
    ///
    /// Returns the path display for sources read from disk and the
    /// synthetic `"<source>"` for sources parsed from in-memory strings.
    pub fn name(&self) -> &str {
        self.file.name()
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
        Self::build(text.to_owned(), "<source>".to_owned())
    }
}


/// Failure to load and parse a Python source file from disk.
#[derive(Debug, Error)]
pub enum SourceError {
    #[error("failed to read {}", path.display())]
    Io {
        path   : PathBuf,
        #[source]
        source : std::io::Error,
    },
    #[error(transparent)]
    Parse(#[from] ParseError),
}


#[cfg(test)]
mod tests {
    use ruff_text_size::TextRange;

    use super::*;

    #[test]
    fn empty_input_parses_as_empty_module() {
        let s = Source::from_str("").expect("empty source parses");
        assert_eq!(s.text(), "");
        assert!(s.ast().body.is_empty());
    }

    #[test]
    fn from_path_missing_file_returns_io_error() {
        let result = Source::from_path(Path::new("/definitely/does/not/exist.py"));
        assert!(matches!(result, Err(SourceError::Io { .. })));
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
        let s   = Source::from_str(src).expect("multibyte source parses");
        let (line, col) = s.line_col(TextSize::new(6));
        assert_eq!(line, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn line_col_handles_unix_newlines() {
        let src = "a\nb\nc\n";
        let s   = Source::from_str(src).expect("LF input parses");
        assert_eq!(s.line_col(TextSize::new(0)), (0, 0));
        assert_eq!(s.line_col(TextSize::new(2)), (1, 0));
        assert_eq!(s.line_col(TextSize::new(4)), (2, 0));
    }

    #[test]
    fn line_col_handles_windows_newlines() {
        let src = "a\r\nb\r\nc\r\n";
        let s   = Source::from_str(src).expect("CRLF input parses");
        assert_eq!(s.line_col(TextSize::new(0)), (0, 0));
        assert_eq!(s.line_col(TextSize::new(3)), (1, 0));
        assert_eq!(s.line_col(TextSize::new(6)), (2, 0));
    }

    #[test]
    fn parse_error_returns_typed_error() {
        let result = Source::from_str("def foo(");
        assert!(matches!(result, Err(ParseError(_))));
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
        let s   = Source::from_str(src).expect("multibyte source parses");
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

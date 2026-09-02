//! Source-text wrapper bundling parsed AST, token stream, and line index.

use std::{path::Path, str::FromStr, sync::OnceLock};

use ruff_diagnostics::Edit;
use ruff_notebook::{CellOffsets, Notebook, NotebookError};
use ruff_python_ast::{ModModule, PySourceType, token::Tokens};
use ruff_python_parser::{ParseError, ParseOptions, Parsed, parse};
use ruff_python_trivia::CommentRanges;
use ruff_source_file::{LineEnding, OneIndexed, SourceFile, SourceFileBuilder, find_newline};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use rustc_hash::FxHashSet;
use thiserror::Error;

use crate::{
    primitives::{
        binding::BindingAnalysis,
        padding::Stranding,
        reserve::{Carry, Columns, Reservations},
    },
    suppression::SuppressionMap,
};

mod brackets;
mod cells;
mod lines;
mod reparse;
#[cfg(test)]
pub(crate) use reparse::slid_range;
mod tables;
pub(crate) mod trace;

/// Owned wrapper around a parsed Python source file.
///
/// Holds the source text, the syntax tree, the token stream, a lazy
/// line index, the `CommentRanges` and `SuppressionMap` indexes derived
/// from that token stream, and the `BindingAnalysis`, alignment-column,
/// and stranded-padding walks each built on first read or carried
/// across a reparse from the source before it. `source_type` is the
/// parse mode and `line_ending` the sequence the text breaks its lines
/// with, leaving `cell_offsets` and `cell_numbers` to carry a
/// notebook's cell boundaries and positions, empty for a module.
#[derive(Debug)]
pub struct Source {
    ast: ModModule,
    binding_analysis: OnceLock<Box<BindingAnalysis>>,
    cell_numbers: Box<[OneIndexed]>,
    cell_offsets: CellOffsets,
    columns: OnceLock<Box<(Reservations, Columns)>>,
    columns_carry: OnceLock<Box<(Reservations, Carry)>>,
    comment_ranges: CommentRanges,
    expandable_literals: OnceLock<Vec<TextRange>>,
    file: SourceFile,
    interpolation_spans: OnceLock<Vec<TextRange>>,
    line_ending: LineEnding,
    paren_followers: OnceLock<FxHashSet<TextSize>>,
    source_type: PySourceType,
    stranded_padding: OnceLock<Box<(Stranding, Vec<Edit>)>>,
    suppression: Box<SuppressionMap>,
    tokens: Tokens,
}

impl Source {
    /// Builds a plain module source around an already-parsed tree, the
    /// probe-side counterpart to [`parsed_module`](Self::parsed_module).
    pub(crate) fn from_parsed_module(text: String, parsed: Parsed<ModModule>) -> Self {
        Self::from_parsed(
            text,
            "<source>",
            PySourceType::default(),
            CellOffsets::default(),
            parsed,
        )
    }

    /// Wraps an already-parsed module in its indexes, splitting the
    /// tree from the token stream this source then owns separately.
    fn from_parsed(
        text: String,
        name: impl Into<Box<str>>,
        source_type: PySourceType,
        cell_offsets: CellOffsets,
        parsed: Parsed<ModModule>,
    ) -> Self {
        let tokens = parsed.tokens().clone();
        Self::from_parts(
            text,
            name,
            source_type,
            cell_offsets,
            parsed.into_syntax(),
            tokens,
        )
    }

    /// Wraps a tree and its token stream in the indexes derived from
    /// them, the shared tail of every constructor and of the
    /// incremental splice.
    fn from_parts(
        text: String,
        name: impl Into<Box<str>>,
        source_type: PySourceType,
        cell_offsets: CellOffsets,
        ast: ModModule,
        tokens: Tokens,
    ) -> Self {
        let comment_ranges = CommentRanges::from(&tokens);
        let line_ending = find_newline(&text).map_or(LineEnding::Lf, |(_, ending)| ending);
        let file = SourceFileBuilder::new(name, text).finish();
        let first_code_offset = ast.body.first().map(Ranged::start);
        let suppression = Box::new(SuppressionMap::from_comments(
            &file.to_source_code(),
            &comment_ranges,
            &tokens,
            first_code_offset,
            &cell_offsets,
        ));
        Self {
            ast,
            binding_analysis: OnceLock::new(),
            cell_numbers: Box::default(),
            cell_offsets,
            columns: OnceLock::new(),
            columns_carry: OnceLock::new(),
            comment_ranges,
            expandable_literals: OnceLock::new(),
            file,
            interpolation_spans: OnceLock::new(),
            line_ending,
            paren_followers: OnceLock::new(),
            source_type,
            stranded_padding: OnceLock::new(),
            suppression,
            tokens,
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

    pub fn ast(&self) -> &ModModule {
        &self.ast
    }

    /// Returns this source's text when it differs from `original`, or
    /// `None` when they match.
    pub fn changed_from(&self, original: &str) -> Option<&str> {
        (self.text() != original).then_some(self.text())
    }

    /// Returns the comment-range index built during parsing.
    pub fn comment_ranges(&self) -> &CommentRanges {
        &self.comment_ranges
    }

    /// Returns `true` when at least one comment lies within `ranged`.
    pub fn intersects_comment<R: Ranged>(&self, ranged: R) -> bool {
        self.comment_ranges.intersects(ranged.range())
    }

    /// Returns the range spanning the entire source text.
    pub fn module_range(&self) -> TextRange {
        TextRange::up_to(self.text().text_len())
    }

    /// Parses Python source from an in-memory string, carrying `name`
    /// the way a file-backed source carries its path.
    ///
    /// # Errors
    ///
    /// Returns the parse error the module parser draws from the text.
    pub fn parse_named(text: String, name: &str) -> Result<Self, ParseError> {
        Self::build_module(text, name, PySourceType::default())
    }

    /// Parses `text` as a plain module, handing back the tree a probe
    /// rebuild clones rather than re-parsing per subset.
    pub(crate) fn parsed_module(text: &str) -> Result<Parsed<ModModule>, ParseError> {
        parse_typed_module(text, PySourceType::default())
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
        next.cell_numbers.clone_from(&self.cell_numbers);
        Ok(next)
    }

    /// Returns the byte slice spanned by anything `Ranged`.
    ///
    /// Accepts a raw `TextRange` or any AST node. The returned `&str`
    /// is guaranteed to fall on `char` boundaries.
    pub fn slice<R: Ranged>(&self, ranged: R) -> &str {
        self.file.slice(ranged.range())
    }

    /// This source's buffer beside its parse mode, the pair
    /// [`build_module`](Self::build_module) rebuilds an equal source
    /// from. `SourceFile` is `Arc`-backed, so holding one is a refcount
    /// bump rather than a copy of the text.
    pub(crate) fn entry_buffer(&self) -> (SourceFile, PySourceType) {
        (self.file.clone(), self.source_type)
    }

    /// Borrows the underlying `SourceFile`.
    pub fn source_file(&self) -> &SourceFile {
        &self.file
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
        &self.tokens
    }
}

/// Clones the text, tree, tokens, and comment indexes, leaving each
/// lazy cache to fill on the copy's own first read.
impl Clone for Source {
    fn clone(&self) -> Self {
        Self {
            ast: self.ast.clone(),
            binding_analysis: OnceLock::new(),
            cell_numbers: self.cell_numbers.clone(),
            cell_offsets: self.cell_offsets.clone(),
            columns: OnceLock::new(),
            columns_carry: OnceLock::new(),
            comment_ranges: self.comment_ranges.clone(),
            expandable_literals: OnceLock::new(),
            file: self.file.clone(),
            interpolation_spans: OnceLock::new(),
            line_ending: self.line_ending,
            paren_followers: OnceLock::new(),
            source_type: self.source_type,
            stranded_padding: OnceLock::new(),
            suppression: self.suppression.clone(),
            tokens: self.tokens.clone(),
        }
    }
}

/// Parses Python source from an in-memory string.
///
/// The resulting `Source` carries the synthetic name `<source>` for
/// diagnostics.
impl FromStr for Source {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse_named(text.to_owned(), "<source>")
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

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::testing::{assert_send_sync, parse, range};

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
    fn changed_from_returns_none_when_text_matches() {
        let s = parse("x = 1\n");
        assert!(s.changed_from("x = 1\n").is_none());
    }

    #[test]
    fn changed_from_returns_text_when_it_differs() {
        let s = parse("x = 1\n");
        assert_eq!(s.changed_from("y = 2\n"), Some("x = 1\n"));
    }

    #[test]
    fn clone_carries_the_text_tree_and_suppression() {
        let source = parse("# prose: off\nx = 1\n");

        let copy = source.clone();

        assert_eq!(copy.text(), source.text());
        assert_eq!(copy.ast().body.len(), source.ast().body.len());
        assert!(copy.suppression_map().file_is_suppressed());
    }

    #[test]
    fn comment_ranges_indexes_each_comment_in_the_source() {
        let s = parse("# top\nx = 1  # trail\n");
        let ranges = s.comment_ranges();
        assert!(ranges.intersects(range(0, 1)));
        assert!(ranges.intersects(range(13, 14)));
    }

    #[test]
    fn empty_input_parses_as_empty_module() {
        let s = parse("");
        assert_eq!(s.text(), "");
        assert!(s.ast().body.is_empty());
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
    fn parse_error_returns_ruff_parse_error() {
        let result: Result<Source, ParseError> = Source::from_str("def foo(");
        assert!(result.is_err());
    }

    #[test]
    fn parse_named_carries_the_name_a_file_backed_source_would() {
        let named =
            Source::parse_named("x = 1\n".to_owned(), "probe.py").expect("a named source parses");

        assert_eq!(named.source_file().name(), "probe.py");
        assert_eq!(parse("x = 1\n").source_file().name(), "<source>");
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
        let s = parse("x = 1\n");
        let result = s.reparse_carrying("def foo(".to_owned(), CellOffsets::default());
        assert!(result.is_err());
    }

    #[test]
    fn single_character_input_parses() {
        let s = parse("x");
        assert_eq!(s.text(), "x");
        assert_eq!(s.ast().body.len(), 1);
    }

    #[test]
    fn slice_accepts_ast_nodes_via_ranged() {
        let s = parse("x = 1\n");
        let stmt = s.ast().body.first().expect("one statement");
        assert_eq!(s.slice(stmt), "x = 1");
    }

    #[test]
    fn slice_at_multibyte_boundary_returns_full_codepoint() {
        let src = "α = 1";
        let s = parse(src);
        let alpha = s.slice(range(0, 2));
        assert_eq!(alpha, "α");
    }

    #[test]
    fn source_is_send_and_sync() {
        assert_send_sync::<Source>();
    }

    #[test]
    fn tokens_returns_non_empty_stream_for_non_empty_source() {
        let s = parse("x = 1");
        assert!(s.tokens().iter().next().is_some());
    }
}

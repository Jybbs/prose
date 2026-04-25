//! Locates positional facts about a `Source`. Wraps upstream
//! primitives (`SourceFile::line_column`,
//! `ruff_python_trivia::leading_indentation`) so rules can share
//! one helper instead of each copying the same call.

use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::TextSize;

use crate::source::Source;

/// Returns the zero-indexed character column of `offset` on its line.
pub fn column_of(source: &Source, offset: TextSize) -> usize {
    source.line_col(offset).column.to_zero_indexed()
}

/// Returns the leading-whitespace prefix of the line containing
/// `offset`. Recognizes Python's full whitespace set (spaces, tabs,
/// form feed, vertical tab) via `ruff_python_trivia`.
pub fn line_indent(source: &Source, offset: TextSize) -> &str {
    leading_indentation(source.text().line_str(offset))
}

/// Returns the character-width of the leading-whitespace prefix on the
/// line containing `offset`. Tabs and form-feeds count as one character
/// each, matching the column semantics used elsewhere in `prose`.
pub fn line_indent_width(source: &Source, offset: TextSize) -> usize {
    line_indent(source, offset).chars().count()
}

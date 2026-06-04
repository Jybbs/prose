//! Triple-quoted docstring body geometry: the inner body slice and
//! the source indent prefix.

use ruff_python_ast::{StringFlags, StringLiteral};
use ruff_python_trivia::{has_leading_content, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::source::Source;

/// Body slice between a triple-quoted docstring's opener and closer,
/// paired with the source range that slice covers.
pub(crate) struct DocstringBody<'a> {
    pub(crate) range: TextRange,
    pub(crate) text: &'a str,
}

impl DocstringBody<'_> {
    /// True when the body spans more than one line.
    pub(crate) fn is_multiline(&self) -> bool {
        self.text.contains('\n')
    }
}

/// Returns the line indent prefix of the docstring at `lit.start()`,
/// preserving the source's mix of tabs and spaces verbatim.
pub(crate) fn indent_prefix<'a>(source: &'a Source, lit: &StringLiteral) -> &'a str {
    leading_indentation(source.text().line_str(lit.start()))
}

/// Returns the body slice and source range when `lit` is triple-quoted
/// and sits at the start of its own line. Returns `None` for
/// non-triple-quoted literals and inline `def f(): """..."""` shapes.
pub(crate) fn triple_quoted_body<'a>(
    source: &'a Source,
    lit: &StringLiteral,
) -> Option<DocstringBody<'a>> {
    if !lit.flags.is_triple_quoted() || has_leading_content(lit.start(), source.text()) {
        return None;
    }
    let range = TextRange::new(
        lit.start() + lit.flags.opener_len(),
        lit.end() - lit.flags.closer_len(),
    );
    Some(DocstringBody {
        range,
        text: source.slice(range),
    })
}

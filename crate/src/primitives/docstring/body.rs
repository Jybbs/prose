//! Docstring body geometry: the inner body slice and the source indent
//! prefix. `docstring_body` accepts any quote style, `triple_quoted_body`
//! narrows to the canonical `"""` form.

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

/// Returns the body slice and source range of the docstring `lit` when
/// it sits at the start of its own line, whatever its quote style.
/// Returns `None` for an inline `def f(): """..."""` shape.
pub(crate) fn docstring_body<'a>(
    source: &'a Source,
    lit: &StringLiteral,
) -> Option<DocstringBody<'a>> {
    if has_leading_content(lit.start(), source.text()) {
        return None;
    }
    let range = lit.content_range();
    Some(DocstringBody {
        range,
        text: source.slice(range),
    })
}

/// Returns the line indent prefix of the docstring at `lit.start()`,
/// preserving the source's mix of tabs and spaces verbatim.
pub(crate) fn indent_prefix<'a>(source: &'a Source, lit: &StringLiteral) -> &'a str {
    leading_indentation(source.text().line_str(lit.start()))
}

/// Returns the body slice of a triple-quoted `lit`, the `"""` or `'''`
/// form `docstring-expand` and `docstring-wrap` act on once
/// `docstring-frame` has canonicalized every docstring to `"""`.
/// Returns `None` for a non-triple-quoted or inline literal.
pub(crate) fn triple_quoted_body<'a>(
    source: &'a Source,
    lit: &StringLiteral,
) -> Option<DocstringBody<'a>> {
    if !lit.flags.is_triple_quoted() {
        return None;
    }
    docstring_body(source, lit)
}

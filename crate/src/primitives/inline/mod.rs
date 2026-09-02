//! The whitespace-folded form of an expression, closing its soft wraps
//! onto one line and declining the one leaf a fold would respace, plus
//! the column measures a rendered form or a source line answers the
//! budget with.

use std::borrow::Cow;

use memchr::memchr;
use ruff_diagnostics::Edit;
use ruff_python_ast::Expr;
use ruff_python_trivia::leading_indentation;
use ruff_source_file::UniversalNewlines;
use ruff_text_size::TextRange;
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{
        padding,
        tokens::{CLOSERS, OPENERS},
        travel::spans_a_string_part,
    },
    source::Source,
};

/// True where a row of `slice` ends on a backslash continuation.
pub(crate) fn carries_a_continuation(slice: &str) -> bool {
    slice
        .universal_newlines()
        .any(|line| line.as_str().trim_end().ends_with('\\'))
}

/// The display width of `text`, its byte length where every byte is
/// ASCII and none is a carriage return.
pub(crate) fn display_width(text: &str) -> usize {
    if text.is_ascii() && memchr(b'\r', text.as_bytes()).is_none() {
        text.len()
    } else {
        text.width()
    }
}

/// The column `text` ends at when its opening line starts at `indent`,
/// measured past the last line break `text` carries.
pub(crate) fn end_column(text: &str, indent: usize) -> usize {
    let last = last_line(text);
    display_width(last) + if last.len() == text.len() { indent } else { 0 }
}

/// `slice`'s single-line form reachable by folding whitespace alone:
/// the borrowed slice when it carries no break and the soft-wrap
/// collapse when `expr` joins operands with an operator. `None` for
/// every other multi-line expression, for one holding a string part
/// that spans rows, for one a backslash continues, and for one hanging
/// a method chain's links.
pub(crate) fn folded_line_form<'s>(
    source: &Source,
    expr: &Expr,
    slice: &'s str,
) -> Option<Cow<'s, str>> {
    if !spans_rows(slice) {
        return Some(Cow::Borrowed(slice));
    }
    (is_operator_tree(expr)
        && !spans_a_string_part(source, expr)
        && !carries_a_continuation(slice)
        && !hangs_a_chain_link(slice))
    .then(|| Cow::Owned(collapse_soft_wraps(slice)))
}

/// The character width of `line`'s leading indentation. Tabs and
/// form-feeds count as one character each.
pub(crate) fn indent_width(line: &str) -> usize {
    leading_indentation(line).chars().count()
}

/// The text past the last line break in `text`, `text` itself where it
/// carries none.
pub(crate) fn last_line(text: &str) -> &str {
    text.rsplit_once(['\n', '\r'])
        .map_or(text, |(_, last)| last)
}

/// The display width of `text`'s opening line.
pub(crate) fn opening_width(text: &str) -> usize {
    display_width(
        text.universal_newlines()
            .next()
            .map_or("", |line| line.as_str()),
    )
}

/// True where the whitespace run covering `[begin, begin + len)` of
/// `text` closes to a single space rather than to nothing, meaning it
/// sits between two tokens rather than directly inside a bracket.
pub(crate) fn run_closes_to_a_space(text: &str, begin: usize, len: usize) -> bool {
    !text[..begin].ends_with(OPENERS) && !text[begin + len..].starts_with(CLOSERS)
}

/// `width`, the display width `range` was measured at, less the padding
/// `padding` drops inside `range`.
pub(crate) fn settled_width(
    source: &Source,
    padding: &[Edit],
    range: TextRange,
    width: usize,
) -> usize {
    width.saturating_add_signed(-padding::slack(source, padding, range))
}

/// The display width `range` settles to once the padding rule drops the
/// delimiter padding and colon padding inside it.
pub(crate) fn settled_slice_width(source: &Source, padding: &[Edit], range: TextRange) -> usize {
    settled_width(source, padding, range, display_width(source.slice(range)))
}

/// The display width `text` settles to: the settled width of `range`
/// where `text` is that source slice as written, and its own width for
/// a rewrite, which carries no padding.
pub(crate) fn settled_text_width(
    source: &Source,
    padding: &[Edit],
    text: &str,
    range: TextRange,
) -> usize {
    if source.slice(range) == text {
        settled_slice_width(source, padding, range)
    } else {
        display_width(text)
    }
}

/// Yields the `(start, len)` byte span of each whitespace run in `text`
/// that spans a line break.
pub(crate) fn soft_wrap_runs(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    whitespace_runs(text).filter(move |&(begin, len)| spans_rows(&text[begin..begin + len]))
}

/// True where `text` spans more than one row under any line ending.
pub(crate) fn spans_rows(text: &str) -> bool {
    text.contains(['\n', '\r'])
}

/// Yields the `(start, len)` byte span of each maximal whitespace run
/// in `text`.
pub(crate) fn whitespace_runs(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut cursor = 0;
    std::iter::from_fn(move || {
        let begin = cursor + text[cursor..].find(char::is_whitespace)?;
        let len = text[begin..]
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(text.len() - begin);
        cursor = begin + len;
        Some((begin, len))
    })
}

/// Collapses each whitespace run that spans a line break, to nothing
/// where the run sits directly inside a bracket and to a single space
/// everywhere else, leaving every run that carries no break as the
/// source wrote it.
fn collapse_soft_wraps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_end = 0;
    for (begin, len) in soft_wrap_runs(text) {
        out.push_str(&text[prev_end..begin]);
        prev_end = begin + len;
        if run_closes_to_a_space(text, begin, len) {
            out.push(' ');
        }
    }
    out.push_str(&text[prev_end..]);
    out
}

/// True where a row of `slice` opens on a `.`, the shape a hung method
/// chain link takes.
fn hangs_a_chain_link(slice: &str) -> bool {
    slice
        .universal_newlines()
        .skip(1)
        .any(|line| line.as_str().trim_start().starts_with('.'))
}

/// True for an expression whose own node joins operands with an
/// operator, whatever its operands.
fn is_operator_tree(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::BinOp(_) | Expr::BoolOp(_) | Expr::Compare(_) | Expr::UnaryOp(_)
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

    #[rstest]
    #[case("a", "a")]
    #[case("a\n    + b", "a + b")]
    #[case("a +  b", "a +  b")]
    #[case("first\n    and second", "first and second")]
    #[case("f(\n    a,\n    b\n)", "f(a, b)")]
    #[case("[\n    a\n]", "[a]")]
    #[case("f(a,\n    b)", "f(a, b)")]
    fn collapse_soft_wraps_folds_only_runs_carrying_a_break(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(collapse_soft_wraps(src), expected);
    }

    #[rstest]
    #[case("", 0)]
    #[case("abc", 3)]
    #[case("a\r\nb", 3)]
    #[case("é", 1)]
    fn display_width_matches_unicode_width_across_the_fast_path_boundary(
        #[case] text: &str,
        #[case] expected: usize,
    ) {
        assert_eq!(display_width(text), expected);
        assert_eq!(display_width(text), text.width());
    }

    #[rstest]
    #[case("", 4, 4)]
    #[case("gamma_value=fn", 4, 18)]
    #[case("head(\n    a\n)  + ", 4, 5)]
    fn end_column_measures_past_the_last_break(
        #[case] text: &str,
        #[case] indent: usize,
        #[case] expected: usize,
    ) {
        assert_eq!(end_column(text, indent), expected);
    }

    #[rstest]
    #[case("", 0)]
    #[case("value", 0)]
    #[case("    value", 4)]
    #[case("\t\tvalue", 2)]
    #[case("\x0c value", 2)]
    fn indent_width_counts_each_whitespace_character_once(
        #[case] line: &str,
        #[case] expected: usize,
    ) {
        assert_eq!(indent_width(line), expected);
    }

    #[rstest]
    #[case("a + b", true)]
    #[case("a and b", true)]
    #[case("a < b", true)]
    #[case("not a", true)]
    #[case("a + helper(b)", true)]
    #[case("greeting + \"!\"", true)]
    #[case("helper(a)", false)]
    #[case("[a, b]", false)]
    #[case("\"a\" \"b\"", false)]
    #[case("value", false)]
    fn is_operator_tree_admits_operator_nodes_whatever_their_operands(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(is_operator_tree(first_expr(&source)), expected);
    }

    #[rstest]
    #[case("(a, b)", 6)]
    #[case("(\"a\", \"\"\"x\ny\"\"\")", 10)]
    #[case("", 0)]
    fn opening_width_stops_at_the_first_break(#[case] text: &str, #[case] expected: usize) {
        assert_eq!(opening_width(text), expected);
    }
}

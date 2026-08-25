//! The whitespace-folded form of an expression, closing its soft wraps
//! onto one line and declining the one leaf a fold would respace, plus
//! the column measures a rendered form or a source line answers the
//! budget with.

use std::borrow::Cow;

use ruff_python_ast::Expr;
use ruff_python_trivia::leading_indentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{
        tokens::{CLOSERS, OPENERS},
        travel::spans_a_string_part,
    },
    source::Source,
};

/// True where a row of `slice` ends on a backslash continuation. The
/// backslash reads as a line break rather than as a token, so a rewrite
/// closing the row behind it strands the backslash ahead of a space and
/// the text stops parsing. `shed-backslash-continuations` takes those
/// out, and a subset running without it still reaches every rule that
/// reshapes a row.
pub(crate) fn carries_a_continuation(slice: &str) -> bool {
    slice.lines().any(|line| line.trim_end().ends_with('\\'))
}

/// True where a row of `slice` opens on a `.`, the shape a hung method
/// chain link takes. Closing that break would draw the dot up onto the
/// row above, which `stack-method-chains` breaks open again from its
/// own link count on the pass that follows.
pub(crate) fn hangs_a_chain_link(slice: &str) -> bool {
    slice
        .lines()
        .skip(1)
        .any(|line| line.trim_start().starts_with('.'))
}

/// The column `text` ends at when its opening line starts at `indent`,
/// measured past the last line break `text` carries.
pub(crate) fn end_column(text: &str, indent: usize) -> usize {
    text.rsplit_once('\n')
        .map_or_else(|| indent + text.width(), |(_, last)| last.width())
}

/// `slice`'s single-line form reachable by folding whitespace alone:
/// the borrowed slice when it carries no break and the soft-wrap
/// collapse when `expr` joins operands with an operator. `None` for
/// every other multi-line expression, each of which a layout rule of
/// its own lays out, for one holding a string part that spans rows,
/// whose interior the collapse would respace, for one a backslash
/// continues, whose break the collapse would close, and for one
/// hanging a method chain's links, whose dots the collapse would draw
/// up a row.
pub(crate) fn folded_line_form<'s>(
    source: &Source,
    expr: &Expr,
    slice: &'s str,
) -> Option<Cow<'s, str>> {
    if !slice.contains('\n') {
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

/// The display width of `text`'s opening line.
pub(crate) fn opening_width(text: &str) -> usize {
    text.lines().next().unwrap_or_default().width()
}

/// True where the whitespace run covering `[begin, begin + len)` of
/// `text` closes to a single space rather than to nothing, meaning it
/// sits between two tokens rather than directly inside a bracket.
pub(crate) fn run_closes_to_a_space(text: &str, begin: usize, len: usize) -> bool {
    !text[..begin].ends_with(OPENERS) && !text[begin + len..].starts_with(CLOSERS)
}

/// Yields the `(start, len)` byte span of each whitespace run in `text`
/// that spans a line break.
pub(crate) fn soft_wrap_runs(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    whitespace_runs(text).filter(move |&(begin, len)| text[begin..begin + len].contains('\n'))
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
/// source wrote it. A run against a delimiter closes to nothing because
/// the space would strand as padding `strip-stranded-padding` takes
/// back on a later pass.
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

/// True for an expression whose own node joins operands with an
/// operator, the shape carrying its soft wraps between operands rather
/// than inside a leaf a rule of its own reshapes. The operands
/// themselves are unconstrained, so a call, a subscript, a collection,
/// and a string literal each ride along inside one.
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

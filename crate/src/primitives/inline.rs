//! The single-line form of a leaf expression, collapsing a soft-wrapped
//! operator-atom tree onto one line, closing a fractured argument list
//! inside it, and declining a leaf whose join would respace a token,
//! plus the column measures a rendered form or a source line answers
//! the budget with.

use std::borrow::Cow;

use ruff_python_ast::{Expr, helpers::is_dotted_name};
use ruff_python_trivia::leading_indentation;
use ruff_text_size::TextRange;
use unicode_width::UnicodeWidthStr;

use crate::{primitives::fracture, source::Source};

/// The column `text` ends at when its opening line starts at `indent`,
/// measured past the last line break `text` carries.
pub(crate) fn end_column(text: &str, indent: usize) -> usize {
    text.rsplit_once('\n')
        .map_or_else(|| indent + text.width(), |(_, last)| last.width())
}

/// `slice`'s single-line form reachable by folding whitespace alone:
/// the borrowed slice when it carries no break, the soft-wrap collapse
/// when `expr` is a break-carrying operator-atom tree, and `None` for a
/// multi-line leaf a fold would split or respace. Every form it returns
/// is reachable by folding whitespace, where [`single_line_form`] also
/// closes a fracture.
pub(crate) fn folded_line_form<'s>(expr: &Expr, slice: &'s str) -> Option<Cow<'s, str>> {
    if !slice.contains('\n') {
        return Some(Cow::Borrowed(slice));
    }
    is_operator_atom_tree(expr).then(|| Cow::Owned(collapse_soft_wraps(slice)))
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

/// `range`'s single-line form, the fold of [`folded_line_form`] and
/// otherwise the fracture close when every break inside `range` shuts.
/// The returned text is the whole replacement, so a caller splices it
/// rather than reproducing it.
pub(crate) fn single_line_form<'s>(
    source: &'s Source,
    rejoin: fracture::Settings,
    expr: &Expr,
    range: TextRange,
) -> Option<Cow<'s, str>> {
    if let Some(folded) = folded_line_form(expr, source.slice(range)) {
        return Some(folded);
    }
    let settled = rejoin.text(source, expr, range);
    (!settled.contains('\n')).then_some(settled)
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

/// Collapses each whitespace run that spans a line break into a single
/// space, leaving every other run as the source wrote it.
fn collapse_soft_wraps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_end = 0;
    for (begin, len) in soft_wrap_runs(text) {
        out.push_str(&text[prev_end..begin]);
        out.push(' ');
        prev_end = begin + len;
    }
    out.push_str(&text[prev_end..]);
    out
}

/// True for an expression built only from binary, boolean, comparison,
/// and unary operators over dotted names and numeric atoms. Such a tree
/// carries no string, call, or bracketed member, so collapsing the
/// whitespace of a soft-wrapped one onto a single line cannot split or
/// respace a token the way a multi-line string or call would.
fn is_operator_atom_tree(expr: &Expr) -> bool {
    match expr {
        Expr::BinOp(b) => is_operator_atom_tree(&b.left) && is_operator_atom_tree(&b.right),
        Expr::BoolOp(b) => b.values.iter().all(is_operator_atom_tree),
        Expr::BooleanLiteral(_) | Expr::NoneLiteral(_) | Expr::NumberLiteral(_) => true,
        Expr::Compare(c) => {
            is_operator_atom_tree(&c.left) && c.comparators.iter().all(is_operator_atom_tree)
        }
        Expr::UnaryOp(u) => is_operator_atom_tree(&u.operand),
        _ => is_dotted_name(expr),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::{
        config::Config,
        testing::{first_expr, parse},
    };

    #[rstest]
    #[case("a", "a")]
    #[case("a\n    + b", "a + b")]
    #[case("a +  b", "a +  b")]
    #[case("first\n    and second", "first and second")]
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
    #[case("a + b * c", true)]
    #[case("(a + b) * c", true)]
    #[case("a and b", true)]
    #[case("a < b <= c", true)]
    #[case("-a + b", true)]
    #[case("module.attr + offset", true)]
    #[case("2 ** depth", true)]
    #[case("a + helper(b)", false)]
    #[case("prefix + values[0]", false)]
    #[case("greeting + \"!\"", false)]
    fn is_operator_atom_tree_accepts_operator_trees_over_atoms_only(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        assert_eq!(is_operator_atom_tree(expr), expected);
    }

    #[rstest]
    #[case("(a, b)", 6)]
    #[case("(\"a\", \"\"\"x\ny\"\"\")", 10)]
    #[case("", 0)]
    fn opening_width_stops_at_the_first_break(#[case] text: &str, #[case] expected: usize) {
        assert_eq!(opening_width(text), expected);
    }

    #[rstest]
    #[case::no_break("value", Some("value"))]
    #[case::operator_soft_wrap("(a +\n    b)", Some("a + b"))]
    #[case::fracture_closes("helper(\n    x)", Some("helper(x)"))]
    #[case::flush_column_holds("helper(\n    x,\n)", None)]
    #[case::multiline_string_holds("\"\"\"x\ny\"\"\"", None)]
    #[case::joined_text_still_spans("helper(\n    \"\"\"x\ny\"\"\")", None)]
    fn single_line_form_collapses_every_break_that_closes(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        let settings = Config::default().fracture_settings();
        assert_eq!(
            single_line_form(&source, settings, expr, expr.range()).as_deref(),
            expected,
        );
    }
}

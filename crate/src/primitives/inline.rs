//! The single-line form of a leaf expression, collapsing a soft-wrapped
//! operator-atom tree onto one line and declining a leaf whose join
//! would respace a token.

use std::borrow::Cow;

use ruff_python_ast::{Expr, helpers::is_dotted_name};

/// Collapses each whitespace run that spans a line break into a single
/// space, leaving runs without a break untouched. Rejoins a soft-wrapped
/// operator expression without respacing its operator tokens, since a run
/// already free of a break is the source's own spacing.
fn collapse_soft_wraps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_end = 0;
    for (begin, len) in whitespace_runs(text) {
        out.push_str(&text[prev_end..begin]);
        let run = &text[begin..begin + len];
        out.push_str(if run.contains('\n') { " " } else { run });
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
        Expr::Compare(c) => {
            is_operator_atom_tree(&c.left) && c.comparators.iter().all(is_operator_atom_tree)
        }
        Expr::UnaryOp(u) => is_operator_atom_tree(&u.operand),
        Expr::NumberLiteral(_) | Expr::BooleanLiteral(_) | Expr::NoneLiteral(_) => true,
        _ => is_dotted_name(expr),
    }
}

/// `expr`'s single-line form when collapsing it respaces no token: the
/// borrowed slice when it carries no break, the soft-wrap collapse when
/// it is a break-carrying operator-atom tree, and `None` for a multi-line
/// leaf a join would split or respace.
pub(crate) fn single_line_form<'s>(expr: &Expr, slice: &'s str) -> Option<Cow<'s, str>> {
    if !slice.contains('\n') {
        return Some(Cow::Borrowed(slice));
    }
    is_operator_atom_tree(expr).then(|| Cow::Owned(collapse_soft_wraps(slice)))
}

/// Yields the `(start, len)` byte span of each maximal whitespace run in
/// `text`, the runs a soft-wrap collapse folds to a single space.
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

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::{first_expr, parse};

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
    #[case("value", Some("value"))]
    #[case("(a +\n    b)", Some("a + b"))]
    #[case("helper(\n    x)", None)]
    fn single_line_form_collapses_atom_breaks_and_declines_the_rest(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let expr = first_expr(&source);
        let slice = source.slice(expr.range());
        assert_eq!(single_line_form(expr, slice).as_deref(), expected);
    }
}

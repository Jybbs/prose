//! Shared layout helpers for the one-per-line expansion and for
//! reading the bracket shape a block already carries.

use std::borrow::Cow;

use ruff_python_ast::Expr;
use ruff_python_trivia::textwrap::{dedent, indent};

use crate::primitives::{INDENT_STEP, inline::indent_width};

/// Builds the one-per-line expansion `(\n<prefix>item,\n…\n<indent>)`
/// for `count` items at `indent`. `render` writes item `i` into the
/// buffer, and `trailing` adds a comma after the last item. Items sit
/// one `INDENT_STEP` past `indent`, the closing `)` at `indent`.
pub(crate) fn explode_parens(
    newline: &str,
    indent: usize,
    count: usize,
    mut render: impl FnMut(&mut String, usize),
    trailing: bool,
) -> String {
    let prefix = " ".repeat(indent + INDENT_STEP);
    let mut out = String::from("(");
    for i in 0..count {
        out.push_str(newline);
        out.push_str(&prefix);
        render(&mut out, i);
        if trailing || i + 1 < count {
            out.push(',');
        }
    }
    out.push_str(newline);
    out.push_str(&prefix[..indent]);
    out.push(')');
    out
}

/// Splits `block` at its first line break when that opening line holds
/// its bracket alone, yielding the bracket and the body beneath. A
/// single-line block and one whose first line carries content beside
/// the bracket both return `None`.
pub(crate) fn flush_bracket_open(block: &str) -> Option<(&str, &str)> {
    let (open, body) = block.split_once('\n')?;
    (open.trim().len() == 1).then_some((open, body))
}

/// True for the four collection-literal `Expr` variants the layout
/// rules lay out, `Dict`, `List`, `Set`, and `Tuple`.
pub(crate) fn is_layoutable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Dict(_) | Expr::List(_) | Expr::Set(_) | Expr::Tuple(_)
    )
}

/// Re-indents the multi-line bracket `block` that `explode_parens` or
/// collection layout emits at one indent so its closing bracket lands
/// at `to`, keeping the body's relative depth. The opening line stays
/// flush, since the caller places it inline after the keyword. The body
/// dedents to its least-indented line, the closing bracket, then
/// re-indents to `to`. Only the exploded form re-indents, so a
/// single-line `block` and one whose opening bracket shares its first
/// line with content both return borrowed. A caller excludes a block
/// whose interior spans a string literal, whose lines `indent` would pad.
pub(crate) fn reindent_block(block: &str, to: usize) -> Cow<'_, str> {
    let Some((open, body)) = flush_bracket_open(block) else {
        return Cow::Borrowed(block);
    };
    Cow::Owned(format!(
        "{open}\n{}",
        indent(&dedent(body), &" ".repeat(to))
    ))
}

/// The columns [`reindent_block`] moves `block`'s body by when it
/// re-indents to `to`, zero where it leaves the block borrowed.
pub(crate) fn reindent_shift(block: &str, to: usize) -> isize {
    match reindent_block(block, to) {
        Cow::Borrowed(_) => 0,
        Cow::Owned(moved) => closing_indent(&moved) as isize - closing_indent(block) as isize,
    }
}

/// The indent width of `block`'s last line.
fn closing_indent(block: &str) -> usize {
    indent_width(block.rsplit_once('\n').map_or(block, |(_, last)| last))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

    #[test]
    fn reindent_block_borrows_a_packed_first_line() {
        // Content beside the opening bracket marks a packed block, not
        // the exploded form, so it holds its source shape.
        assert_matches!(
            reindent_block("(a, b,\n    c)", 8),
            Cow::Borrowed("(a, b,\n    c)")
        );
    }

    #[test]
    fn reindent_block_borrows_a_single_line_block() {
        assert_matches!(reindent_block("{a: b}", 4), Cow::Borrowed("{a: b}"));
    }

    #[rstest]
    #[case("{\n    a,\n    b,\n}", 4, "{\n        a,\n        b,\n    }")]
    #[case("{\n        a,\n    }", 0, "{\n    a,\n}")]
    #[case("{\n        a,\n    }", 4, "{\n        a,\n    }")]
    #[case(
        "[\n    a,\n    [\n        b,\n    ],\n]",
        4,
        "[\n        a,\n        [\n            b,\n        ],\n    ]"
    )]
    #[case("{\n    a,\n\n    b,\n}", 4, "{\n        a,\n\n        b,\n    }")]
    fn reindent_block_shifts_body_to_target_keeping_relative_depth(
        #[case] block: &str,
        #[case] to: usize,
        #[case] expected: &str,
    ) {
        assert_eq!(reindent_block(block, to), expected);
    }

    #[rstest]
    #[case("(a, b,\n    c)", 8, 0)]
    #[case("{a: b}", 4, 0)]
    #[case("{\n    a,\n    b,\n}", 4, 4)]
    #[case("{\n        a,\n    }", 0, -4)]
    fn reindent_shift_reports_the_columns_the_body_moves(
        #[case] block: &str,
        #[case] to: usize,
        #[case] expected: isize,
    ) {
        assert_eq!(reindent_shift(block, to), expected);
    }
}

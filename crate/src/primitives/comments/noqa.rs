//! The `noqa` marker a trailing comment carries, read for the codes it
//! names. A marker suppresses nothing in Prose, and a rule reads one
//! only where the code names a fact about the line that no static read
//! reaches, an import kept for its re-export or held at its position.

use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;

use super::trailing_comment;
use crate::source::Source;

/// The marker a suppression comment opens with, matched case-insensitively.
const NOQA: &str = "noqa";

/// True where a `noqa` comment trails `stmt`, either bare or naming
/// `code`, the bare form covering every code. A statement spanning
/// several rows carries its marker on the row it opens or the row it
/// closes, so both are read.
pub(crate) fn noqa_names(source: &Source, stmt: &Stmt, code: &str) -> bool {
    let marks = |offset| {
        trailing_comment(source, offset).is_some_and(|range| {
            noqa_codes(source.slice(range)).is_some_and(|codes| {
                codes.is_empty() || codes.iter().any(|named| named.eq_ignore_ascii_case(code))
            })
        })
    };
    marks(stmt.start()) || (source.contains_line_break(stmt) && marks(stmt.end()))
}

/// True for a `flake8`-shaped code, one or more ASCII letters closed by
/// one or more ASCII digits. The first token failing this opens the
/// prose an author writes after the codes.
fn is_rule_code(code: &str) -> bool {
    let letters = code.trim_end_matches(|c: char| c.is_ascii_digit());
    !letters.is_empty()
        && letters.len() < code.len()
        && letters.chars().all(|c| c.is_ascii_alphabetic())
}

/// The codes a `noqa` comment names, reading tokens until one fails the
/// code shape and the rest reads as prose. Empty for the bare form
/// covering every code, `None` where the comment carries no `noqa`.
fn noqa_codes(comment: &str) -> Option<Vec<&str>> {
    let opened = marker_end(comment)?;
    let Some(listed) = comment[opened..].trim_start().strip_prefix(':') else {
        return Some(Vec::new());
    };
    Some(
        listed
            .split([',', ' ', '\t'])
            .filter(|code| !code.is_empty())
            .take_while(|code| is_rule_code(code))
            .collect(),
    )
}

/// The offset just past the `noqa` marker of `comment`, which opens a
/// comment rather than sitting in its prose, and which the codes or the
/// end of the comment follow. A stacked comment opens a fresh hash, so
/// every hash is tried.
fn marker_end(comment: &str) -> Option<usize> {
    comment.match_indices('#').find_map(|(at, _)| {
        let rest = comment[at + 1..].trim_start();
        let opened = comment.len() - rest.len();
        let named = rest
            .as_bytes()
            .get(..NOQA.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(NOQA.as_bytes()));
        let closed = rest
            .as_bytes()
            .get(NOQA.len())
            .is_none_or(|&byte| byte == b':' || byte.is_ascii_whitespace());
        (named && closed).then_some(opened + NOQA.len())
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn a_marker_closing_a_multi_row_statement_reads() {
        let source = parse("from os import (\n    getcwd,\n)  # noqa: F401\n");
        assert!(noqa_names(&source, &source.ast().body[0], "F401"));
    }

    #[test]
    fn a_marker_on_one_statement_leaves_its_neighbour_alone() {
        let source = parse("import os  # noqa: F401\nimport sys\n");
        let body = &source.ast().body;
        assert!(noqa_names(&source, &body[0], "F401"));
        assert!(!noqa_names(&source, &body[1], "F401"));
    }

    #[rstest]
    #[case::bare("  # noqa", "F401", true)]
    #[case::listed("  # noqa: F401", "F401", true)]
    #[case::unspaced("  # noqa:F401", "F401", true)]
    #[case::lowercase("  # noqa: f401", "F401", true)]
    #[case::among_others("  # noqa: E501, F401", "F401", true)]
    #[case::uppercase_marker("  # NOQA: F401", "F401", true)]
    #[case::trailing_prose("  # noqa: F401 kept for re-export", "F401", true)]
    #[case::position_code("  # noqa: E402", "E402", true)]
    #[case::other_code("  # noqa: E501", "F401", false)]
    #[case::other_code_with_prose("  # noqa: E501 line is long", "F401", false)]
    #[case::space_separated("  # noqa: E501 F401", "F401", true)]
    #[case::named_in_prose("  # we cannot use noqa here", "F401", false)]
    #[case::marker_is_a_prefix("  # noqable", "F401", false)]
    #[case::stacked_pragma("  # type: ignore  # noqa: F401", "F401", true)]
    #[case::plain_comment("  # kept for re-export", "F401", false)]
    #[case::no_comment("", "F401", false)]
    fn noqa_names_reads_the_marker_and_its_codes(
        #[case] comment: &str,
        #[case] code: &str,
        #[case] names: bool,
    ) {
        let source = parse(&format!("import os{comment}\n"));
        assert_eq!(noqa_names(&source, &source.ast().body[0], code), names);
    }
}

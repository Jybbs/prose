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
/// `code`, the bare form covering every code.
pub(crate) fn noqa_names(source: &Source, stmt: &Stmt, code: &str) -> bool {
    trailing_comment(source, stmt.start()).is_some_and(|range| {
        noqa_codes(source.slice(range)).is_some_and(|codes| {
            codes.is_empty() || codes.iter().any(|named| named.eq_ignore_ascii_case(code))
        })
    })
}

/// The codes a `noqa` comment names, empty for the bare form covering
/// every code, `None` where the comment carries no `noqa` at all.
fn noqa_codes(comment: &str) -> Option<Vec<&str>> {
    let lowered = comment.to_ascii_lowercase();
    let opened = lowered.find(NOQA)? + NOQA.len();
    let Some(listed) = comment[opened..].trim_start().strip_prefix(':') else {
        return Some(Vec::new());
    };
    Some(
        listed
            .split(',')
            .map(str::trim)
            .take_while(|code| !code.is_empty() && code.chars().all(char::is_alphanumeric))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

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

    #[test]
    fn a_marker_on_one_statement_leaves_its_neighbour_alone() {
        let source = parse("import os  # noqa: F401\nimport sys\n");
        let body = &source.ast().body;
        assert!(noqa_names(&source, &body[0], "F401"));
        assert!(!noqa_names(&source, &body[1], "F401"));
    }
}

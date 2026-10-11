//! Parse-surface tests, the directives one comment's text carries.

use std::assert_matches;

use rstest::rstest;
use ruff_python_trivia::SuppressionKind;

use crate::suppression::parse::{directives, is_directive_comment, is_keep_marker};

#[test]
fn first_format_directive_on_a_comment_wins() {
    let found = directives("# prose: off # prose: on");
    assert_matches!(found.region, Some(SuppressionKind::Off));
}

#[rstest]
#[case("# fmt: off", true)]
#[case("# prose: skip[align-equals]", true)]
#[case("# prose: ignore", true)]
#[case("# prose: keep", false)]
#[case("# a plain note", false)]
fn is_directive_comment_spots_format_and_lint_directives(
    #[case] comment: &str,
    #[case] expected: bool,
) {
    assert_eq!(is_directive_comment(comment), expected);
}

#[rstest]
#[case::spaced("# prose: keep", true)]
#[case::tight("#prose:keep", true)]
#[case::doubled_hash("## prose: keep", true)]
#[case::after_another_chunk("# noqa  # prose: keep", true)]
#[case::trailing_words("# prose: keep the order", false)]
#[case::longer_word("# prose: keeps", false)]
#[case::bare_word("# keep", false)]
fn is_keep_marker_reads_a_prose_keep_chunk(#[case] comment: &str, #[case] expected: bool) {
    assert_eq!(is_keep_marker(comment), expected);
}

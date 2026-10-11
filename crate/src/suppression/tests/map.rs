//! Map-surface tests, the spans, regions, and per-line directives a
//! source's suppression index records and the queries it answers.

use std::assert_matches;

use rstest::rstest;

use super::line;
use crate::{
    rules::{RuleId, align_equals::AlignEquals, alphabetize_siblings::AlphabetizeSiblings},
    suppression::RuleEntry,
    testing::{at, notebook, parse, range},
};

#[test]
fn a_bare_skip_after_a_listed_one_widens_to_every_rule() {
    let source = parse("x = 1  # prose: skip[align-equals]  # prose: skip\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 5), AlignEquals::SLUG));
    assert!(map.suppresses(range(0, 5), AlphabetizeSiblings::SLUG));
}

#[test]
fn a_fmt_skip_widens_a_listed_prose_skip_on_its_comment() {
    let source = parse("x = 1  # prose: skip[align-equals]  # fmt: skip\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 5), AlphabetizeSiblings::SLUG));
    assert_matches!(map.skips(), [(_, skip)] if matches!(skip.named.entry, RuleEntry::All));
}

#[test]
fn a_listed_skip_beside_an_off_still_opens_the_region() {
    let source = parse("# prose: skip[align-equals]  # prose: off\naa = 1\nb = 2\n");
    assert!(source.suppression_map().file_is_suppressed());
}

#[rstest]
fn bare_or_listed_ignore_suppresses_each_named_rule(
    #[values(
        "x = 1  # prose: ignore\n",
        "x = 1  # prose: ignore  # prose: ignore[align-equals]\n",
        "x = 1  # prose: ignore[align-equals, alphabetize-siblings]\n",
        "x = 1  # prose: ignore[align-equals]  # prose: ignore\n",
        "x = 1  # prose: ignore[align-equals]  # prose: ignore[alphabetize-siblings]\n"
    )]
    src: &str,
) {
    let source = parse(src);
    let map = source.suppression_map();
    assert!(map.is_lint_suppressed_at(line(0), AlignEquals::SLUG));
    assert!(map.is_lint_suppressed_at(line(0), AlphabetizeSiblings::SLUG));
}

#[test]
fn bare_prose_skip_opens_a_full_line_span() {
    let source = parse("x = 1  # prose: skip\n");
    let map = source.suppression_map();
    assert!(map.has_format_suppression());
    assert!(map.suppresses(range(0, 6), AlignEquals::SLUG));
    // A bare skip opens no off region, so a lint on the line survives.
    assert!(!map.intersects(range(0, 6)));
}

#[test]
fn empty_source_yields_empty_map() {
    let source = parse("");
    let map = source.suppression_map();
    assert!(!map.intersects(range(0, 1)));
    assert!(!map.intersects(range(0, 0)));
    assert!(!map.has_format_suppression());
    assert!(!map.has_lint_suppression());
    assert!(!map.file_is_suppressed());
}

#[rstest]
fn file_is_suppressed_by_an_unmatched_off_ahead_of_the_code(
    #[values(
        "# leading note\n\n# prose: off\nx = 1\n",
        "# prose: off\nx = 1\ny = 2\n",
        "# fmt: off\nx = 1\n",
        "# yapf: disable\nx = 1\n"
    )]
    src: &str,
) {
    assert!(parse(src).suppression_map().file_is_suppressed());
}

#[rstest]
fn file_not_suppressed_by_an_off_after_code_or_a_matched_one(
    #[values(
        "x = 1\n# prose: off\ny = 2\n",
        "# prose: off\nx = 1\n# prose: on\ny = 2\n"
    )]
    src: &str,
) {
    assert!(!parse(src).suppression_map().file_is_suppressed());
}

#[test]
fn foreign_pragmas_are_invisible() {
    let source = parse(
        "x = 1  # noqa: F401\ny = 2  # type: ignore[name-defined]\nz = 3  # pyright: ignore\n",
    );
    let map = source.suppression_map();
    assert!(!map.has_lint_suppression());
    assert!(!map.has_format_suppression());
    assert!(!map.is_lint_suppressed_at(line(0), AlignEquals::SLUG));
    assert!(!map.is_lint_suppressed_at(line(1), AlignEquals::SLUG));
    assert!(!map.is_lint_suppressed_at(line(2), AlignEquals::SLUG));
}

#[test]
fn ignore_at_reads_the_directive_on_its_line_alone() {
    let source = parse("x = 1\ny = 2  # prose: ignore[align-equals]\n");
    let map = source.suppression_map();
    assert!(map.ignore_at(line(0)).is_none());
    let ignore = map.ignore_at(line(1)).expect("line 2 carries an ignore");
    assert_eq!(
        &source.text()[ignore.comment],
        "# prose: ignore[align-equals]"
    );
    assert!(ignore.named.entry.matches(AlignEquals::SLUG));
}

#[test]
fn intersects_catches_edit_that_fully_contains_a_span() {
    let text = "# fmt: off\nx = 1\n# fmt: on\n";
    let source = parse(text);
    let map = source.suppression_map();
    // Edit spanning the entire suppressed block (offsets 0..27)
    // overlaps the span and must be dropped.
    assert!(map.intersects(range(0, 27)));
}

#[rstest]
fn malformed_directive_does_not_register(
    #[values(
        "x = 1  # prose: ignore[align-equals\n",
        "x = 1  # prose:\n",
        "x = 1  # proseignore\n",
        "x = 1  # prose: ignoring\n",
        "x = 1  # prose: ignore extra\n",
        "x = 1  # prose: skip[align-equals\n",
        "x = 1  # prose: skip extra\n"
    )]
    src: &str,
) {
    let source = parse(src);
    let map = source.suppression_map();
    assert!(!map.has_lint_suppression());
    assert!(!map.has_format_suppression());
}

#[test]
fn mismatched_id_does_not_suppress_the_queried_rule() {
    let source = parse("x = 1  # prose: ignore[align-equals]\n");
    let map = source.suppression_map();
    assert!(!map.is_lint_suppressed_at(line(0), AlphabetizeSiblings::SLUG));
}

#[test]
fn multiple_skip_directives_on_one_comment_union_their_rules() {
    let source = parse("x = 1  # prose: skip[align-equals]  # prose: skip[alphabetize-siblings]\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 5), AlignEquals::SLUG));
    assert!(map.suppresses(range(0, 5), AlphabetizeSiblings::SLUG));
}

#[test]
fn nested_directive_after_non_pragma_hash_is_recognized() {
    let source = parse("x = 1  # my note # prose: ignore\n");
    let map = source.suppression_map();
    assert!(map.is_lint_suppressed_at(line(0), AlignEquals::SLUG));
}

#[test]
fn nested_prose_off_after_non_pragma_hash_is_recognized() {
    let source = parse("# my note # prose: off\nx = 1\n");
    assert!(
        source
            .suppression_map()
            .intersects(at(source.text(), "x = 1"))
    );
}

#[test]
fn off_at_the_top_of_a_single_cell_notebook_suppresses_the_file() {
    let source = notebook(&["# prose: off\nx = 1"]);
    assert!(source.suppression_map().file_is_suppressed());
}

#[test]
fn own_line_skip_at_end_of_file_stays_on_its_own_line() {
    let source = parse("x = 1\n# fmt: skip");
    let map = source.suppression_map();
    assert!(!map.suppresses(at(source.text(), "x = 1"), AlignEquals::SLUG));
}

#[test]
fn own_line_skip_stays_on_its_own_line() {
    let source = parse("x = 1\n# fmt: skip\ny = 2\n");
    let map = source.suppression_map();
    assert!(!map.suppresses(at(source.text(), "x = 1"), AlignEquals::SLUG));
    assert!(!map.suppresses(at(source.text(), "y = 2"), AlignEquals::SLUG));
}

#[rstest]
#[case::skipped_line("x = 1  # prose: skip\n", "x = 1", true)]
#[case::closing_line(
    "# prose: off\nx = 1\n# prose: on\ny = 2\n",
    "# prose: on\ny = 2",
    true
)]
#[case::below_the_close("# prose: off\nx = 1\n# prose: on\ny = 2\n", "y = 2", false)]
#[case::stray_on("# prose: on\ny = 2\n", "# prose: on\ny = 2", false)]
#[case::other_rule("x = 1  # prose: skip[align-equals]\n", "x = 1", false)]
fn pins_a_suppressed_span_and_the_line_closing_a_region(
    #[case] src: &str,
    #[case] needle: &str,
    #[case] expected: bool,
) {
    let source = parse(src);
    let pinned = source
        .suppression_map()
        .pins(at(source.text(), needle), AlphabetizeSiblings::SLUG);
    assert_eq!(pinned, expected);
}

#[rstest]
fn prose_off_and_fmt_off_open_the_same_span(
    #[values(
        "# prose: off\nx = 1\n# prose: on\n",
        "# fmt: off\nx = 1\n# fmt: on\n",
        "# prose: off\nx = 1\n",
        "# fmt: off\nx = 1\n"
    )]
    text: &str,
) {
    let src = parse(text);
    assert!(src.suppression_map().intersects(at(src.text(), "x = 1")));
}

#[test]
fn rule_skip_on_a_wrapped_statement_reaches_its_opening_line() {
    let source = parse("z = (\n    x\n)  # prose: skip[alphabetize-siblings]\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 1), AlphabetizeSiblings::SLUG));
    assert!(!map.suppresses(range(0, 1), AlignEquals::SLUG));
}

#[rstest]
fn single_id_suppresses_exactly_the_listed_rule(
    #[values(
        "x = 1  # prose: ignore[align-equals]\n",
        "x = 1  # prose: ignore[align-equals, not-a-rule]\n"
    )]
    src: &str,
) {
    let source = parse(src);
    let map = source.suppression_map();
    assert!(map.is_lint_suppressed_at(line(0), AlignEquals::SLUG));
    assert!(!map.is_lint_suppressed_at(line(0), AlphabetizeSiblings::SLUG));
}

#[test]
fn skip_after_a_backslash_continuation_reaches_the_opening_line() {
    let source = parse("x = 1 + \\\n    2  # fmt: skip\ny = 3\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 1), AlignEquals::SLUG));
    assert!(!map.suppresses(at(source.text(), "y = 3"), AlignEquals::SLUG));
}

#[rstest]
fn skip_brackets_target_only_listed_rules(
    #[values(
        "x = 1  # prose: skip[align-equals]\n",
        "x = 1  # prose: skip[align-equals, not-a-rule]\n"
    )]
    src: &str,
) {
    let source = parse(src);
    let map = source.suppression_map();
    assert!(map.has_format_suppression());
    assert!(map.suppresses(range(0, 5), AlignEquals::SLUG));
    assert!(!map.suppresses(range(0, 5), AlphabetizeSiblings::SLUG));
}

#[test]
fn skip_in_a_notebook_cell_spans_its_logical_line() {
    let source = notebook(&["z = (\n    x\n)  # fmt: skip", "y = 2"]);
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 1), AlignEquals::SLUG));
    assert!(!map.suppresses(at(source.text(), "y = 2"), AlignEquals::SLUG));
}

#[test]
fn skip_inside_a_bracketed_construct_stays_on_its_own_line() {
    let source = parse("config = {\n    \"a\": 1,  # fmt: skip\n    \"b\": 2,\n}\n");
    let map = source.suppression_map();
    assert!(map.suppresses(at(source.text(), "\"a\""), AlignEquals::SLUG));
    assert!(!map.suppresses(range(0, 6), AlignEquals::SLUG));
}

#[test]
fn skip_multi_id_suppresses_each_listed_rule() {
    let source = parse("x = 1  # prose: skip[align-equals, alphabetize-siblings]\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 5), AlignEquals::SLUG));
    assert!(map.suppresses(range(0, 5), AlphabetizeSiblings::SLUG));
}

#[test]
fn skip_on_a_compound_header_stops_at_the_body() {
    let source = parse("if (\n    ready\n):  # fmt: skip\n    pass\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 2), AlignEquals::SLUG));
    assert!(!map.suppresses(at(source.text(), "pass"), AlignEquals::SLUG));
}

#[test]
fn skip_on_a_wrapped_statement_reaches_its_opening_line() {
    let source = parse("z = (\n    x\n)  # fmt: skip\n");
    assert!(
        source
            .suppression_map()
            .suppresses(range(0, 1), AlignEquals::SLUG)
    );
}

#[rstest]
#[case::trailing_skip_alone("x = 1  # fmt: skip\ny = 2\n", "x = 1\ny = 2\n")]
#[case::own_line_skip("x = 1\n# prose: skip\ny = 2\n", "x = 1\ny = 2\n")]
#[case::doubled_hash("x = 1  ## prose: skip\n", "x = 1\n")]
#[case::skip_before_a_note("x = 1  # fmt: skip  # noqa: E501\n", "x = 1  # noqa: E501\n")]
#[case::skip_after_a_note(
    "x = 1  # noqa: E501  # prose: skip[align-equals]\n",
    "x = 1  # noqa: E501\n"
)]
#[case::skip_between_notes("x = 1  # a  # fmt: skip  # prose: skip  # b\n", "x = 1  # a  # b\n")]
fn skip_removal_deletes_the_skip_chunks_and_the_space_around_them(
    #[case] src: &str,
    #[case] expected: &str,
) {
    let source = parse(src);
    let [(_, skip)] = source.suppression_map().skips() else {
        panic!("one skip directive");
    };
    let (without, _) = source
        .deleting(&skip.skip_removal(source.text()))
        .expect("the shortened source parses");
    assert_eq!(without.text(), expected);
}

#[test]
fn skip_span_opens_at_the_statement_below_a_comment_gap() {
    let source = parse("a = 1\n\n# note\nz = (\n    x\n)  # fmt: skip\n");
    let map = source.suppression_map();
    assert!(map.suppresses(at(source.text(), "z"), AlignEquals::SLUG));
    assert!(!map.suppresses(range(0, 5), AlignEquals::SLUG));
    assert!(!map.suppresses(at(source.text(), "# note"), AlignEquals::SLUG));
}

#[test]
fn skip_span_survives_crlf_line_endings() {
    let source = parse("z = (\r\n    x\r\n)  # fmt: skip\r\ny = 2\r\n");
    let map = source.suppression_map();
    assert!(map.suppresses(range(0, 1), AlignEquals::SLUG));
    assert!(!map.suppresses(at(source.text(), "y = 2"), AlignEquals::SLUG));
}

#[rstest]
#[case(AlignEquals::SLUG)]
#[case(AlphabetizeSiblings::SLUG)]
fn skip_whitespace_tolerant_inside_brackets(#[case] rule: RuleId) {
    let canonical = parse("x = 1  # prose: skip[align-equals, alphabetize-siblings]\n");
    let compact = parse("x = 1  # prose:skip[ align-equals ,alphabetize-siblings ]\n");
    let canonical_map = canonical.suppression_map();
    let compact_map = compact.suppression_map();
    assert_eq!(
        canonical_map.suppresses(range(0, 5), rule),
        compact_map.suppresses(range(0, 5), rule),
    );
    assert!(canonical_map.suppresses(range(0, 5), rule));
}

#[rstest]
#[case::bare_skip("x = 1  # prose: skip\ny = 2\n", true)]
#[case::listed_skip("x = 1  # prose: skip[align-equals]\ny = 2\n", true)]
#[case::no_skip("x = 1\ny = 2  # prose: skip\n", false)]
fn skipped_reads_a_skip_whatever_rules_it_names(#[case] src: &str, #[case] expected: bool) {
    let source = parse(src);
    assert_eq!(source.suppression_map().skipped(range(0, 6)), expected);
}

#[test]
fn the_unknown_ids_a_directive_names_are_kept_as_written() {
    let source = parse(
        "x = 1  # prose: skip[no-such-rule , align-equals]  # prose: ignore[typo,]  # prose: ignore[other-typo]\n",
    );
    let map = source.suppression_map();
    let [(_, skip)] = map.skips() else {
        panic!("one skip directive");
    };
    assert_eq!(skip.named.unknown, [Box::from("no-such-rule")]);
    assert!(skip.named.entry.matches(AlignEquals::SLUG));
    let ignore = map.ignore_at(line(0)).expect("the line carries an ignore");
    assert_eq!(
        ignore.named.unknown,
        [Box::from("typo"), Box::from("other-typo")]
    );
    assert_matches!(&ignore.named.entry, RuleEntry::Specific(rules) if rules.is_empty());
}

#[test]
fn trailing_comment_directive_suppresses_its_line() {
    let source = parse("x = 1  # prose: ignore\n");
    let map = source.suppression_map();
    assert!(map.has_lint_suppression());
    assert!(map.is_lint_suppressed_at(line(0), AlignEquals::SLUG));
}

#[test]
fn trailing_prose_off_does_not_open_a_format_span() {
    let source = parse("x = 1  # prose: off\ny = 2\n");
    let map = source.suppression_map();
    assert!(!map.has_format_suppression());
    assert!(!map.file_is_suppressed());
}

#[test]
fn unmatched_off_in_a_notebook_closes_at_its_cell_end() {
    // `# prose: off` opens in cell 0, so it suppresses that cell's `x`
    // but not cell 1's `y`, and the file is not wholly suppressed.
    let source = notebook(&["# prose: off\nx = 1", "y = 2"]);
    let map = source.suppression_map();
    assert!(map.intersects(at(source.text(), "x")));
    assert!(!map.intersects(at(source.text(), "y")));
    assert!(!map.file_is_suppressed());
}

#[rstest]
#[case(AlignEquals::SLUG)]
#[case(AlphabetizeSiblings::SLUG)]
fn whitespace_tolerant_canonical_and_compact_forms_parse_identically(#[case] rule: RuleId) {
    let canonical = parse("x = 1  # prose: ignore[align-equals, alphabetize-siblings]\n");
    let compact = parse("x = 1  # prose:ignore[ align-equals ,alphabetize-siblings ]\n");
    let canonical_map = canonical.suppression_map();
    let compact_map = compact.suppression_map();
    assert_eq!(
        canonical_map.is_lint_suppressed_at(line(0), rule),
        compact_map.is_lint_suppressed_at(line(0), rule),
    );
    assert!(canonical_map.is_lint_suppressed_at(line(0), rule));
}

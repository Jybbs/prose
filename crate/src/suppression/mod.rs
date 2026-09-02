//! Per-`Source` index of `# prose: off` / `# prose: on` / `# prose: skip`
//! spans (plus the `# fmt:` and `# yapf:` aliases), `# prose: skip[<id>]`
//! per-rule format directives, and `# prose: ignore[<id>]` per-line lint
//! directives. Built once during `Source` construction and consulted by
//! `Pipeline::run` to drop suppressed fix groups and `Severity::Lint`
//! diagnostics. A skip directive that closes its logical line spans
//! every physical line that line occupies and suppresses rewrites
//! alone, leaving lint diagnostics to the `ignore` directives, whereas
//! an off region suppresses both. A `file_is_suppressed` shortcut lets
//! the pipeline skip rule execution entirely when an unmatched off
//! precedes every statement.

use memchr::memchr;
use ruff_notebook::CellOffsets;
use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_python_trivia::{CommentLinePosition, CommentRanges, SuppressionKind};
use ruff_source_file::{LineRanges, OneIndexed, SourceCode};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{primitives::range::merged_spans, rule::RuleId};

mod format_directive;
mod lint_directive;
mod parse_common;

use format_directive::{FormatDirective, parse_format};
use lint_directive::{RuleEntry, parse_ignore};
use parse_common::after_prose_prefix;

/// Sorted byte-range lists for the `# prose: off` regions and the bare
/// `# prose: skip` spans, paired with the `# prose: skip[<id>]` per-rule
/// spans and a per-line `OneIndexed` map of `# prose: ignore` lint
/// directives. An off region suppresses rewrites and lint diagnostics
/// alike, whereas a skip span suppresses rewrites alone and leaves lints
/// to the `ignore` directives. Span queries run in O(log n) against
/// `spans` and `skip_spans`, O(n) against `skips`, and O(1) per line.
#[derive(Clone, Debug)]
pub(crate) struct SuppressionMap {
    file_suppressed: bool,
    lints: FxHashMap<OneIndexed, RuleEntry>,
    skip_spans: Vec<TextRange>,
    skips: Vec<(TextRange, RuleEntry)>,
    spans: Vec<TextRange>,
}

impl SuppressionMap {
    /// Walks `comments` against `source`, indexing the off regions, the
    /// bare skip spans, the per-rule skip spans, and the per-line lint
    /// directives. `tokens` resolves each skip directive's logical line,
    /// and `first_code_offset` is the start of the source's first
    /// top-level statement, or `None` for code-free input.
    pub(crate) fn from_comments(
        source: &SourceCode<'_, '_>,
        comments: &CommentRanges,
        tokens: &Tokens,
        first_code_offset: Option<TextSize>,
        cell_offsets: &CellOffsets,
    ) -> Self {
        let source_text = source.text();
        let mut lints: FxHashMap<OneIndexed, RuleEntry> = FxHashMap::default();
        let mut skip_spans: Vec<TextRange> = Vec::new();
        let mut skips: Vec<(TextRange, RuleEntry)> = Vec::new();
        let mut spans: Vec<TextRange> = Vec::new();
        let mut open_off: Option<TextSize> = None;
        for range in comments {
            let comment = &source_text[range];
            let found = directives(comment);
            if let Some(directive) = found.format {
                let position = CommentLinePosition::for_range(range, source_text);
                match directive {
                    FormatDirective::Kind(SuppressionKind::Off) if position.is_own_line() => {
                        open_off.get_or_insert_with(|| source_text.line_start(range.start()));
                    }
                    FormatDirective::Kind(SuppressionKind::On) if position.is_own_line() => {
                        spans.extend(open_off.take().map(|start| {
                            TextRange::new(start, source_text.line_start(range.start()))
                        }));
                    }
                    FormatDirective::Kind(SuppressionKind::Skip) => {
                        skip_spans.push(skip_span(source_text, tokens, range));
                    }
                    FormatDirective::SkipRules(rules) => {
                        skips.push((
                            skip_span(source_text, tokens, range),
                            RuleEntry::Specific(rules),
                        ));
                    }
                    FormatDirective::Kind(_) => {}
                }
            }
            if let Some(entry) = found.lint {
                let line = source.line_index(range.start());
                lints.entry(line).or_default().merge(entry);
            }
        }
        let unmatched_span = open_off
            .map(|start| TextRange::new(start, cell_close_end(cell_offsets, source_text, start)));
        let file_suppressed = unmatched_span.is_some_and(|span| {
            span.end() == source_text.text_len()
                && first_code_offset.is_none_or(|code| span.start() <= code)
        });
        spans.extend(unmatched_span);
        Self {
            file_suppressed,
            lints,
            skip_spans: merged_spans(skip_spans),
            skips,
            spans: merged_spans(spans),
        }
    }

    /// Returns `true` when an unmatched `# prose: off` (or alias) sits
    /// at or before the first non-blank, non-comment line of the file.
    pub(crate) fn file_is_suppressed(&self) -> bool {
        self.file_suppressed
    }

    /// Returns `true` when the source carries at least one off region,
    /// bare skip span, or `# prose: skip[<id>]` directive.
    pub(crate) fn has_format_suppression(&self) -> bool {
        !self.spans.is_empty() || !self.skip_spans.is_empty() || !self.skips.is_empty()
    }

    /// Returns `true` when the source carries at least one
    /// `# prose: ignore` directive.
    pub(crate) fn has_lint_suppression(&self) -> bool {
        !self.lints.is_empty()
    }

    /// Returns `true` when `ranged`'s span overlaps a `# prose: off`
    /// region by at least one byte. Empty ranges report overlap when
    /// their offset strictly sits inside a region. A bare
    /// `# prose: skip` opens no region, so it does not report here and
    /// lint diagnostics on its line survive.
    pub(crate) fn intersects<R: Ranged>(&self, ranged: R) -> bool {
        covers(&self.spans, ranged.range())
    }

    /// Returns `true` when `line` carries a `# prose: ignore`
    /// directive that suppresses `rule`. Bare directives suppress
    /// every rule on their line.
    pub(crate) fn is_lint_suppressed_at(&self, line: OneIndexed, rule: RuleId) -> bool {
        self.lints.get(&line).is_some_and(|e| e.matches(rule))
    }

    /// Returns `true` when `ranged` overlaps a `# prose: off` region, a
    /// bare `# prose: skip` span, or a `# prose: skip[<id>]` span
    /// listing `rule`.
    pub(crate) fn suppresses<R: Ranged>(&self, ranged: R, rule: RuleId) -> bool {
        let range = ranged.range();
        covers(&self.spans, range)
            || covers(&self.skip_spans, range)
            || self
                .skips
                .iter()
                .any(|(span, entry)| span.ordering(range).is_eq() && entry.matches(rule))
    }
}

/// The format and lint directives one comment carries.
#[derive(Default)]
struct Directives {
    format: Option<FormatDirective>,
    lint: Option<RuleEntry>,
}

/// True when `comment` is a recognized format or lint directive, so it
/// drives suppression from its own line and must stay pinned there
/// rather than move with a sibling reorder.
pub(crate) fn is_directive_comment(comment: &str) -> bool {
    let found = directives(comment);
    found.format.is_some() || found.lint.is_some()
}

/// The offset an unmatched `# prose: off` opened at `start` closes at:
/// the end of the notebook cell holding `start`, or the buffer's end for
/// an ordinary module whose `cell_offsets` are empty.
fn cell_close_end(cell_offsets: &CellOffsets, source_text: &str, start: TextSize) -> TextSize {
    cell_offsets
        .containing_range(start)
        .map_or(source_text.text_len(), TextRange::end)
}

/// Returns `true` when `range` overlaps one of the sorted, merged
/// `spans` by at least one byte.
fn covers(spans: &[TextRange], range: TextRange) -> bool {
    spans.binary_search_by(|s| s.ordering(range)).is_ok()
}

/// Parses `comment` once for both directive families. Each `#` chunk
/// carrying the `prose:` prefix feeds whichever family its body names,
/// a later `skip[<id>]` or `ignore[<id>]` chunk unioning its ids into
/// the first, and the `# fmt:` and `# yapf:` aliases stand in where no
/// `prose:` format directive is present. A comment carrying no `:`
/// holds neither and skips the walk.
fn directives(comment: &str) -> Directives {
    let mut found = Directives::default();
    if memchr(b':', comment.as_bytes()).is_none() {
        return found;
    }
    for body in comment.split('#').skip(1).filter_map(after_prose_prefix) {
        if let Some(next) = parse_format(body) {
            match (&mut found.format, next) {
                (Some(FormatDirective::SkipRules(rules)), FormatDirective::SkipRules(more)) => {
                    rules.extend(more);
                }
                (Some(_), _) => {}
                (slot @ None, next) => *slot = Some(next),
            }
        }
        if let Some(entry) = parse_ignore(body) {
            found.lint.get_or_insert_default().merge(entry);
        }
    }
    if found.format.is_none() {
        found.format = SuppressionKind::from_comment(comment).map(FormatDirective::Kind);
    }
    found
}

/// The span a skip directive occupying `comment` suppresses.
///
/// A directive closing its logical line covers that line from its first
/// non-trivia token through the comment's own physical line. A directive
/// whose logical line runs on past it covers its physical line alone.
fn skip_span(source_text: &str, tokens: &Tokens, comment: TextRange) -> TextRange {
    let physical = source_text.full_line_range(comment.start());
    let closes_line = tokens
        .after(comment.end())
        .first()
        .is_none_or(|token| token.kind() == TokenKind::Newline);
    if !closes_line {
        return physical;
    }
    let anchor = tokens
        .before(comment.start())
        .rsplit(|token| token.kind() == TokenKind::Newline)
        .next()
        .and_then(|line| line.iter().find(|token| !token.kind().is_trivia()))
        .map_or(comment.start(), Token::start);
    TextRange::new(source_text.line_start(anchor), physical.end())
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;
    use ruff_source_file::OneIndexed;

    use super::{FormatDirective, SuppressionKind, directives, is_directive_comment};
    use crate::{
        rule::RuleId,
        rules::{align_equals::AlignEquals, alphabetize_siblings::AlphabetizeSiblings},
        testing::{at, notebook, parse, range},
    };

    fn line(zero_indexed: usize) -> OneIndexed {
        OneIndexed::from_zero_indexed(zero_indexed)
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
    fn first_format_directive_on_a_comment_wins() {
        let found = directives("# prose: off # prose: on");
        assert_matches!(
            found.format,
            Some(FormatDirective::Kind(SuppressionKind::Off))
        );
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
    fn intersects_catches_edit_that_fully_contains_a_span() {
        let text = "# fmt: off\nx = 1\n# fmt: on\n";
        let source = parse(text);
        let map = source.suppression_map();
        // Edit spanning the entire suppressed block (offsets 0..27)
        // overlaps the span and must be dropped.
        assert!(map.intersects(range(0, 27)));
    }

    #[rstest]
    #[case("# fmt: off", true)]
    #[case("# prose: skip[align-equals]", true)]
    #[case("# prose: ignore", true)]
    #[case("# a plain note", false)]
    fn is_directive_comment_spots_format_and_lint_directives(
        #[case] comment: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_directive_comment(comment), expected);
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
        let source =
            parse("x = 1  # prose: skip[align-equals]  # prose: skip[alphabetize-siblings]\n");
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
        assert!(!source.suppression_map().intersects(range(0, 5)));
    }

    #[test]
    fn own_line_skip_stays_on_its_own_line() {
        let source = parse("x = 1\n# fmt: skip\ny = 2\n");
        let map = source.suppression_map();
        assert!(!map.intersects(at(source.text(), "x = 1")));
        assert!(!map.intersects(at(source.text(), "y = 2")));
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
}

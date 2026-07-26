//! Per-`Source` index of `# prose: off` / `# prose: on` / `# prose: skip`
//! spans (plus the `# fmt:` and `# yapf:` aliases), `# prose: skip[<id>]`
//! per-rule format directives, and `# prose: ignore[<id>]` per-line lint
//! directives. Built once during `Source` construction and consulted by
//! `Pipeline::run` to drop suppressed fix groups and `Severity::Lint`
//! diagnostics. A skip directive that closes its logical line spans
//! every physical line that line occupies. A `file_is_suppressed`
//! shortcut lets the pipeline skip rule execution entirely when an
//! unmatched off precedes every statement.

use std::collections::HashMap;

use ruff_notebook::CellOffsets;
use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_python_trivia::{CommentLinePosition, CommentRanges, SuppressionKind};
use ruff_source_file::{LineRanges, OneIndexed, SourceCode};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::rule::RuleId;

mod format_directive;
mod lint_directive;
mod parse_common;

use format_directive::{FormatDirective, classify_format_directive};
use lint_directive::{RuleEntry, find_prose_ignore};

/// Sorted byte-range list for format-suppression spans, paired with the
/// `# prose: skip[<id>]` per-rule spans and a per-line `OneIndexed` map
/// of `# prose: ignore` lint directives. Span queries run in O(log n)
/// against `spans` and O(n) against `skips`, per-line lint queries in
/// O(1).
#[derive(Debug)]
pub(crate) struct SuppressionMap {
    file_suppressed: bool,
    lints: HashMap<OneIndexed, RuleEntry>,
    skips: Vec<(TextRange, RuleEntry)>,
    spans: Vec<TextRange>,
}

impl SuppressionMap {
    /// Walks `comments` against `source`, classifying each comment via
    /// `classify_format_directive` for the format spans and the per-rule
    /// skip index, and `find_prose_ignore` for the lint index. `tokens`
    /// resolves each skip directive's logical line through `skip_span`.
    /// `first_code_offset` carries the start of the source's first
    /// top-level statement (or `None` for code-free input), powering
    /// the `file_is_suppressed` shortcut.
    ///
    /// An unmatched `# prose: off` (or alias) extends through end of
    /// file in a module and through the end of its own cell in a
    /// notebook, a stray `# prose: on` is a no-op, and two consecutive
    /// `# prose: off` directives flatten with the first `# prose: on`
    /// closing the block. Multiple `# prose: ignore` directives on the
    /// same line merge with bare-wins precedence, and `# prose: skip[<id>]`
    /// directives union their listed ids.
    pub(crate) fn from_comments(
        source: &SourceCode<'_, '_>,
        comments: &CommentRanges,
        tokens: &Tokens,
        first_code_offset: Option<TextSize>,
        cell_offsets: &CellOffsets,
    ) -> Self {
        let source_text = source.text();
        let mut lints: HashMap<OneIndexed, RuleEntry> = HashMap::new();
        let mut skips: Vec<(TextRange, RuleEntry)> = Vec::new();
        let mut spans: Vec<TextRange> = Vec::new();
        let mut open_off: Option<TextSize> = None;
        for range in comments {
            let comment = &source_text[range];
            if let Some(directive) = classify_format_directive(comment) {
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
                        spans.push(skip_span(source_text, tokens, range));
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
            if let Some(entry) = find_prose_ignore(comment) {
                let line = source.line_index(range.start());
                lints.entry(line).or_default().merge(entry);
            }
        }
        let off_end = open_off.map(|start| cell_close_end(cell_offsets, source_text, start));
        let file_suppressed = open_off.zip(off_end).is_some_and(|(off, end)| {
            end == source_text.text_len() && first_code_offset.is_none_or(|code| off <= code)
        });
        spans.extend(
            open_off
                .zip(off_end)
                .map(|(start, end)| TextRange::new(start, end)),
        );
        Self {
            file_suppressed,
            lints,
            skips,
            spans: merge_spans(spans),
        }
    }

    /// Returns `true` when an unmatched `# prose: off` (or alias) sits
    /// at or before the first non-blank, non-comment line of the file.
    pub(crate) fn file_is_suppressed(&self) -> bool {
        self.file_suppressed
    }

    /// Returns `true` when the source carries at least one
    /// format-suppression span or `# prose: skip[<id>]` directive.
    pub(crate) fn has_format_directive(&self) -> bool {
        !self.spans.is_empty() || !self.skips.is_empty()
    }

    /// Returns `true` when the source carries at least one
    /// `# prose: ignore` directive.
    pub(crate) fn has_lint_suppression(&self) -> bool {
        !self.lints.is_empty()
    }

    /// Returns `true` when `ranged`'s span overlaps any
    /// format-suppressed span by at least one byte. Empty ranges
    /// report overlap when their offset strictly sits inside a span.
    pub(crate) fn intersects<R: Ranged>(&self, ranged: R) -> bool {
        self.spans
            .binary_search_by(|s| s.ordering(ranged.range()))
            .is_ok()
    }

    /// Returns `true` when `line` carries a `# prose: ignore`
    /// directive that suppresses `rule`. Bare directives suppress
    /// every rule on their line.
    pub(crate) fn is_lint_suppressed_at(&self, line: OneIndexed, rule: RuleId) -> bool {
        self.lints.get(&line).is_some_and(|e| e.matches(rule))
    }

    /// Returns `true` when `ranged` overlaps a format-suppressed span
    /// or a `# prose: skip[<id>]` span listing `rule`.
    pub(crate) fn suppresses<R: Ranged>(&self, ranged: R, rule: RuleId) -> bool {
        let range = ranged.range();
        self.intersects(range)
            || self
                .skips
                .iter()
                .any(|(span, entry)| span.ordering(range).is_eq() && entry.matches(rule))
    }
}

/// True when `comment` is a recognized format or lint directive, so it
/// drives suppression from its own line and must stay pinned there
/// rather than ride a sibling reorder.
pub(crate) fn is_directive_comment(comment: &str) -> bool {
    classify_format_directive(comment).is_some() || find_prose_ignore(comment).is_some()
}

/// The offset an unmatched `# prose: off` opened at `start` closes at:
/// the end of the notebook cell holding `start`, or the buffer's end for
/// an ordinary module whose `cell_offsets` are empty.
fn cell_close_end(cell_offsets: &CellOffsets, source_text: &str, start: TextSize) -> TextSize {
    cell_offsets
        .containing_range(start)
        .map_or(source_text.text_len(), TextRange::end)
}

fn merge_spans(mut spans: Vec<TextRange>) -> Vec<TextRange> {
    spans.sort_unstable_by_key(Ranged::start);
    spans.dedup_by(|next, prev| {
        let overlaps = next.start() <= prev.end();
        if overlaps {
            *prev = prev.cover(*next);
        }
        overlaps
    });
    spans
}

/// The span a skip directive occupying `comment` suppresses.
///
/// A directive closing its logical line covers that whole line, opening
/// at the first non-trivia token after the preceding logical newline and
/// closing at the end of the comment's own physical line, so a wrapped
/// statement or compound header pins every physical line it spans. A
/// directive whose logical line runs on past it, sitting inside a
/// bracketed construct or on its own line, covers its physical line
/// alone.
fn skip_span(source_text: &str, tokens: &Tokens, comment: TextRange) -> TextRange {
    let physical = source_text.full_line_range(comment.start());
    let closes_line = tokens
        .after(comment.end())
        .first()
        .is_none_or(|token| token.kind() == TokenKind::Newline);
    if !closes_line {
        return physical;
    }
    let before = tokens.before(comment.start());
    let line_open = before
        .iter()
        .rposition(|token| token.kind() == TokenKind::Newline)
        .map_or(0, |index| index + 1);
    let anchor = before[line_open..]
        .iter()
        .find(|token| !token.kind().is_trivia())
        .map_or(comment.start(), Token::start);
    TextRange::new(source_text.line_start(anchor), physical.end())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_source_file::OneIndexed;

    use super::is_directive_comment;
    use crate::rule::RuleId;
    use crate::testing::{notebook, parse, range};

    fn align_equals() -> RuleId {
        "align-equals".parse().expect("align-equals is registered")
    }

    fn alphabetize() -> RuleId {
        "alphabetize".parse().expect("alphabetize is registered")
    }

    fn line(zero_indexed: usize) -> OneIndexed {
        OneIndexed::from_zero_indexed(zero_indexed)
    }

    #[test]
    fn bare_ignore_suppresses_every_rule_on_the_line() {
        let source = parse("x = 1  # prose: ignore\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn bare_prose_skip_opens_a_full_line_span() {
        let source = parse("x = 1  # prose: skip\n");
        let map = source.suppression_map();
        assert!(map.has_format_directive());
        assert!(map.intersects(range(0, 6)));
    }

    #[test]
    fn bare_then_specific_keeps_all_suppression() {
        let source = parse("x = 1  # prose: ignore  # prose: ignore[align-equals]\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn empty_source_yields_empty_map() {
        let source = parse("");
        let map = source.suppression_map();
        assert!(!map.intersects(range(0, 1)));
        assert!(!map.intersects(range(0, 0)));
        assert!(!map.has_format_directive());
        assert!(!map.has_lint_suppression());
        assert!(!map.file_is_suppressed());
    }

    #[test]
    fn file_is_suppressed_when_off_precedes_only_blank_and_comment_lines() {
        let source = parse("# leading note\n\n# prose: off\nx = 1\n");
        assert!(source.suppression_map().file_is_suppressed());
    }

    #[test]
    fn file_is_suppressed_when_unmatched_off_sits_at_top() {
        let source = parse("# prose: off\nx = 1\ny = 2\n");
        assert!(source.suppression_map().file_is_suppressed());
    }

    #[test]
    fn file_is_suppressed_with_fmt_off_alias() {
        let source = parse("# fmt: off\nx = 1\n");
        assert!(source.suppression_map().file_is_suppressed());
    }

    #[test]
    fn file_is_suppressed_with_yapf_disable_alias() {
        let source = parse("# yapf: disable\nx = 1\n");
        assert!(source.suppression_map().file_is_suppressed());
    }

    #[test]
    fn file_not_suppressed_when_off_follows_code() {
        let source = parse("x = 1\n# prose: off\ny = 2\n");
        assert!(!source.suppression_map().file_is_suppressed());
    }

    #[test]
    fn file_not_suppressed_when_top_off_has_matching_on() {
        let source = parse("# prose: off\nx = 1\n# prose: on\ny = 2\n");
        assert!(!source.suppression_map().file_is_suppressed());
    }

    #[test]
    fn foreign_pragmas_are_invisible() {
        let source = parse(
            "x = 1  # noqa: F401\ny = 2  # type: ignore[name-defined]\nz = 3  # pyright: ignore\n",
        );
        let map = source.suppression_map();
        assert!(!map.has_lint_suppression());
        assert!(!map.has_format_directive());
        assert!(!map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(!map.is_lint_suppressed_at(line(1), align_equals()));
        assert!(!map.is_lint_suppressed_at(line(2), align_equals()));
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
        assert!(!map.has_format_directive());
    }

    #[test]
    fn mismatched_id_does_not_suppress_the_queried_rule() {
        let source = parse("x = 1  # prose: ignore[align-equals]\n");
        let map = source.suppression_map();
        assert!(!map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn multi_id_suppresses_each_listed_rule() {
        let source = parse("x = 1  # prose: ignore[align-equals, alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn multiple_skip_directives_on_one_comment_union_their_rules() {
        let source = parse("x = 1  # prose: skip[align-equals]  # prose: skip[alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.suppresses(range(0, 5), align_equals()));
        assert!(map.suppresses(range(0, 5), alphabetize()));
    }

    #[test]
    fn nested_directive_after_non_pragma_hash_is_recognized() {
        let source = parse("x = 1  # my note # prose: ignore\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
    }

    #[test]
    fn nested_prose_off_after_non_pragma_hash_is_recognized() {
        let source = parse("# my note # prose: off\nx = 1\n");
        let x_offset = source.text().find('x').expect("x is present") as u32;
        assert!(
            source
                .suppression_map()
                .intersects(range(x_offset, x_offset + 5)),
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
        let offset = |needle: char| source.text().find(needle).expect("present") as u32;
        assert!(!map.intersects(range(offset('x'), offset('x') + 5)));
        assert!(!map.intersects(range(offset('y'), offset('y') + 5)));
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
        let x_offset = src.text().find('x').expect("x is present") as u32;
        assert!(
            src.suppression_map()
                .intersects(range(x_offset, x_offset + 5))
        );
    }

    #[test]
    fn rule_skip_on_a_wrapped_statement_reaches_its_opening_line() {
        let source = parse("z = (\n    x\n)  # prose: skip[alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.suppresses(range(0, 1), alphabetize()));
        assert!(!map.suppresses(range(0, 1), align_equals()));
    }

    #[test]
    fn second_bare_directive_widens_first_specific_to_all() {
        let source = parse("x = 1  # prose: ignore[align-equals]  # prose: ignore\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn single_id_suppresses_exactly_the_listed_rule() {
        let source = parse("x = 1  # prose: ignore[align-equals]\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(!map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn skip_after_a_backslash_continuation_reaches_the_opening_line() {
        let source = parse("x = 1 + \\\n    2  # fmt: skip\ny = 3\n");
        let map = source.suppression_map();
        let y = source.text().find('y').expect("y is present") as u32;
        assert!(map.intersects(range(0, 1)));
        assert!(!map.intersects(range(y, y + 5)));
    }

    #[test]
    fn skip_brackets_target_only_listed_rules() {
        let source = parse("x = 1  # prose: skip[align-equals]\n");
        let map = source.suppression_map();
        assert!(map.has_format_directive());
        assert!(map.suppresses(range(0, 5), align_equals()));
        assert!(!map.suppresses(range(0, 5), alphabetize()));
    }

    #[test]
    fn skip_in_a_notebook_cell_spans_its_logical_line() {
        let source = notebook(&["z = (\n    x\n)  # fmt: skip", "y = 2"]);
        let map = source.suppression_map();
        let y = source.text().find('y').expect("y is present") as u32;
        assert!(map.intersects(range(0, 1)));
        assert!(!map.intersects(range(y, y + 5)));
    }

    #[test]
    fn skip_inside_a_bracketed_construct_stays_on_its_own_line() {
        let source = parse("config = {\n    \"a\": 1,  # fmt: skip\n    \"b\": 2,\n}\n");
        let map = source.suppression_map();
        let entry = source.text().find("\"a\"").expect("first entry is present") as u32;
        assert!(map.intersects(range(entry, entry + 3)));
        assert!(!map.intersects(range(0, 6)));
    }

    #[test]
    fn skip_multi_id_suppresses_each_listed_rule() {
        let source = parse("x = 1  # prose: skip[align-equals, alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.suppresses(range(0, 5), align_equals()));
        assert!(map.suppresses(range(0, 5), alphabetize()));
    }

    #[test]
    fn skip_on_a_compound_header_stops_at_the_body() {
        let source = parse("if (\n    ready\n):  # fmt: skip\n    pass\n");
        let map = source.suppression_map();
        let body = source.text().find("pass").expect("pass is present") as u32;
        assert!(map.intersects(range(0, 2)));
        assert!(!map.intersects(range(body, body + 4)));
    }

    #[test]
    fn skip_on_a_wrapped_statement_reaches_its_opening_line() {
        let source = parse("z = (\n    x\n)  # fmt: skip\n");
        assert!(source.suppression_map().intersects(range(0, 1)));
    }

    #[test]
    fn skip_span_opens_at_the_statement_below_a_comment_gap() {
        let source = parse("a = 1\n\n# note\nz = (\n    x\n)  # fmt: skip\n");
        let map = source.suppression_map();
        let offset = |needle: &str| source.text().find(needle).expect("present") as u32;
        assert!(map.intersects(range(offset("z"), offset("z") + 1)));
        assert!(!map.intersects(range(0, 5)));
        assert!(!map.intersects(range(offset("# note"), offset("# note") + 6)));
    }

    #[test]
    fn skip_span_survives_crlf_line_endings() {
        let source = parse("z = (\r\n    x\r\n)  # fmt: skip\r\ny = 2\r\n");
        let map = source.suppression_map();
        let y = source.text().find('y').expect("y is present") as u32;
        assert!(map.intersects(range(0, 1)));
        assert!(!map.intersects(range(y, y + 5)));
    }

    #[test]
    fn skip_unknown_id_is_dropped_silently() {
        let source = parse("x = 1  # prose: skip[align-equals, not-a-rule]\n");
        let map = source.suppression_map();
        assert!(map.suppresses(range(0, 5), align_equals()));
        assert!(!map.suppresses(range(0, 5), alphabetize()));
    }

    #[rstest]
    #[case(align_equals())]
    #[case(alphabetize())]
    fn skip_whitespace_tolerant_inside_brackets(#[case] rule: RuleId) {
        let canonical = parse("x = 1  # prose: skip[align-equals, alphabetize]\n");
        let compact = parse("x = 1  # prose:skip[ align-equals ,alphabetize ]\n");
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
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
    }

    #[test]
    fn trailing_prose_off_does_not_open_a_format_span() {
        let source = parse("x = 1  # prose: off\ny = 2\n");
        let map = source.suppression_map();
        assert!(!map.has_format_directive());
        assert!(!map.file_is_suppressed());
    }

    #[test]
    fn two_specifics_on_same_line_union_their_ids() {
        let source = parse("x = 1  # prose: ignore[align-equals]  # prose: ignore[alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn unknown_id_is_dropped_silently() {
        let source = parse("x = 1  # prose: ignore[align-equals, not-a-rule]\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
        assert!(!map.is_lint_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn unmatched_off_in_a_notebook_closes_at_its_cell_end() {
        // `# prose: off` opens in cell 0, so it suppresses that cell's `x`
        // but not cell 1's `y`, and the file is not wholly suppressed.
        let source = notebook(&["# prose: off\nx = 1", "y = 2"]);
        let map = source.suppression_map();
        let offset = |needle: char| source.text().find(needle).expect("present") as u32;
        let x = offset('x');
        let y = offset('y');
        assert!(map.intersects(range(x, x + 1)));
        assert!(!map.intersects(range(y, y + 1)));
        assert!(!map.file_is_suppressed());
    }

    #[rstest]
    #[case(align_equals())]
    #[case(alphabetize())]
    fn whitespace_tolerant_canonical_and_compact_forms_parse_identically(#[case] rule: RuleId) {
        let canonical = parse("x = 1  # prose: ignore[align-equals, alphabetize]\n");
        let compact = parse("x = 1  # prose:ignore[ align-equals ,alphabetize ]\n");
        let canonical_map = canonical.suppression_map();
        let compact_map = compact.suppression_map();
        assert_eq!(
            canonical_map.is_lint_suppressed_at(line(0), rule),
            compact_map.is_lint_suppressed_at(line(0), rule),
        );
        assert!(canonical_map.is_lint_suppressed_at(line(0), rule));
    }
}

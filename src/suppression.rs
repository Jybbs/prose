//! Per-`Source` index of `# fmt: off` / `# fmt: on` / `# fmt: skip`
//! spans (plus the `# yapf: disable` / `# yapf: enable` aliases) and
//! `# prose: ignore[...]` per-line lint directives. Built once during
//! `Source` construction and consulted by `Pipeline::run` to drop
//! suppressed edits and `Severity::Lint` diagnostics.

use std::collections::{HashMap, HashSet};

use ruff_python_trivia::{CommentLinePosition, CommentRanges, SuppressionKind};
use ruff_source_file::{LineRanges, OneIndexed, SourceCode};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::rule::RuleId;

/// One line's parsed `# prose: ignore` directive.
#[derive(Debug)]
enum IgnoreEntry {
    /// Bare `# prose: ignore`. Suppresses every rule on the line.
    All,
    /// `# prose: ignore[<id>[, <id>...]]`. Unknown ids are dropped.
    Specific(HashSet<RuleId>),
}

impl IgnoreEntry {
    /// Folds `incoming` into `self`. `All` widens any prior `Specific`,
    /// and a second `Specific` unions its ids into the first.
    fn merge(&mut self, incoming: Self) {
        match (&mut *self, incoming) {
            (IgnoreEntry::All, _) => {}
            (slot @ IgnoreEntry::Specific(_), IgnoreEntry::All) => *slot = IgnoreEntry::All,
            (IgnoreEntry::Specific(rules), IgnoreEntry::Specific(more)) => rules.extend(more),
        }
    }
}

impl Default for IgnoreEntry {
    fn default() -> Self {
        Self::Specific(HashSet::new())
    }
}

/// Sorted byte-range list for format-suppression spans paired with a
/// per-line `OneIndexed` map for `# prose: ignore` directives. Format
/// queries run in O(log n), lint queries in O(1).
#[derive(Debug)]
pub(crate) struct SuppressionMap {
    lints: HashMap<OneIndexed, IgnoreEntry>,
    spans: Vec<TextRange>,
}

impl SuppressionMap {
    /// Walks `comments` against `source`, classifying each comment via
    /// `SuppressionKind` for the format spans and `find_prose_directive`
    /// for the lint index. An unmatched `# fmt: off` extends through
    /// end of file. A stray `# fmt: on` is a no-op. Two consecutive
    /// `# fmt: off` directives flatten, with the first `# fmt: on`
    /// closing the block. Multiple `# prose: ignore` directives on the
    /// same line merge with bare-wins precedence.
    pub(crate) fn from_comments(source: &SourceCode<'_, '_>, comments: &CommentRanges) -> Self {
        let source_text = source.text();
        let mut lints: HashMap<OneIndexed, IgnoreEntry> = HashMap::new();
        let mut spans: Vec<TextRange> = Vec::new();
        let mut open_off: Option<TextSize> = None;
        for range in comments {
            let comment = &source_text[range];
            if let Some(kind) = SuppressionKind::from_comment(comment) {
                let position = CommentLinePosition::for_range(range, source_text);
                match kind {
                    SuppressionKind::Off if position.is_own_line() => {
                        open_off.get_or_insert_with(|| source_text.line_start(range.start()));
                    }
                    SuppressionKind::On if position.is_own_line() => {
                        spans.extend(open_off.take().map(|start| {
                            TextRange::new(start, source_text.line_start(range.start()))
                        }));
                    }
                    SuppressionKind::Skip => {
                        spans.push(source_text.full_line_range(range.start()));
                    }
                    _ => {}
                }
            }
            if let Some(entry) = find_prose_directive(comment) {
                let line = source.line_index(range.start());
                lints.entry(line).or_default().merge(entry);
            }
        }
        spans.extend(open_off.map(|start| TextRange::new(start, source_text.text_len())));
        Self {
            lints,
            spans: merge(spans),
        }
    }

    /// Returns `true` when the source carries at least one
    /// format-suppression span.
    pub(crate) fn has_format_suppression(&self) -> bool {
        !self.spans.is_empty()
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
        let range = ranged.range();
        self.spans.binary_search_by(|s| s.ordering(range)).is_ok()
    }

    /// Returns `true` when `line` carries a `# prose: ignore`
    /// directive that suppresses `rule`. Bare directives suppress
    /// every rule on their line.
    pub(crate) fn is_lint_suppressed_at(&self, line: OneIndexed, rule: RuleId) -> bool {
        self.lints.get(&line).is_some_and(|entry| match entry {
            IgnoreEntry::All => true,
            IgnoreEntry::Specific(rules) => rules.contains(&rule),
        })
    }
}

/// Splits `comment` at each `#` boundary, parsing each chunk as a
/// `prose: ignore` directive and folding successful hits through
/// `merge_entries`. Catches nested forms like `# my note # prose:
/// ignore` and multi-directive lines like `# prose: ignore  # prose:
/// ignore[align-equals]`.
fn find_prose_directive(comment: &str) -> Option<IgnoreEntry> {
    comment
        .split('#')
        .skip(1)
        .filter_map(parse_prose_ignore)
        .reduce(|mut acc, next| {
            acc.merge(next);
            acc
        })
}

fn merge(mut spans: Vec<TextRange>) -> Vec<TextRange> {
    spans.sort_unstable_by_key(|s| s.start());
    spans.dedup_by(|next, prev| {
        let overlaps = next.start() <= prev.end();
        if overlaps {
            *prev = prev.cover(*next);
        }
        overlaps
    });
    spans
}

/// Parses the post-`#` body of a `prose: ignore`, `prose: ignore[<id>]`,
/// or `prose: ignore[<id>, <id>...]` directive. Returns `None` for any
/// other shape. Whitespace tolerated around `:`, `[`, `,`, and `]`.
/// Unknown rule ids inside the brackets are dropped.
fn parse_prose_ignore(after_hash: &str) -> Option<IgnoreEntry> {
    let body = after_hash
        .trim()
        .strip_prefix("prose:")?
        .trim()
        .strip_prefix("ignore")?
        .trim();
    if body.is_empty() {
        return Some(IgnoreEntry::All);
    }
    Some(IgnoreEntry::Specific(
        body.strip_prefix('[')?
            .strip_suffix(']')?
            .split(',')
            .filter_map(|part| part.trim().parse::<RuleId>().ok())
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use ruff_source_file::OneIndexed;

    use crate::rule::RuleId;
    use crate::test_support::{parse, range};

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
        assert!(!map.has_format_suppression());
        assert!(!map.has_lint_suppression());
    }

    #[test]
    fn foreign_pragmas_are_invisible() {
        let source = parse(
            "x = 1  # noqa: F401\ny = 2  # type: ignore[name-defined]\nz = 3  # pyright: ignore\n",
        );
        let map = source.suppression_map();
        assert!(!map.has_lint_suppression());
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

    #[test]
    fn malformed_directive_does_not_register() {
        for src in [
            "x = 1  # prose: ignore[align-equals\n",
            "x = 1  # prose:\n",
            "x = 1  # proseignore\n",
            "x = 1  # prose: ignoring\n",
            "x = 1  # prose: ignore extra\n",
        ] {
            let source = parse(src);
            assert!(
                !source.suppression_map().has_lint_suppression(),
                "expected no lint suppression for {src:?}",
            );
        }
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
    fn nested_directive_after_non_pragma_hash_is_recognized() {
        let source = parse("x = 1  # my note # prose: ignore\n");
        let map = source.suppression_map();
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
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
    fn trailing_comment_directive_suppresses_its_line() {
        let source = parse("x = 1  # prose: ignore\n");
        let map = source.suppression_map();
        assert!(map.has_lint_suppression());
        assert!(map.is_lint_suppressed_at(line(0), align_equals()));
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
    fn whitespace_tolerant_canonical_and_compact_forms_parse_identically() {
        let canonical = parse("x = 1  # prose: ignore[align-equals, alphabetize]\n");
        let compact = parse("x = 1  # prose:ignore[ align-equals ,alphabetize ]\n");
        let canonical_map = canonical.suppression_map();
        let compact_map = compact.suppression_map();
        for rule in [align_equals(), alphabetize()] {
            assert_eq!(
                canonical_map.is_lint_suppressed_at(line(0), rule),
                compact_map.is_lint_suppressed_at(line(0), rule),
            );
            assert!(canonical_map.is_lint_suppressed_at(line(0), rule));
        }
    }
}

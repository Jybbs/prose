//! Per-`Source` index of `# prose: off` / `# prose: on` / `# prose: skip`
//! spans (plus the `# fmt:` and `# yapf:` aliases), `# prose: skip[<id>]`
//! per-line format-rule directives, and `# prose: ignore[<id>]` per-line
//! lint directives. Built once during `Source` construction and consulted
//! by `Pipeline::run` to drop suppressed edits and `Severity::Lint`
//! diagnostics. A `file_is_suppressed` shortcut lets the pipeline
//! skip rule execution entirely when an unmatched off precedes every
//! statement.

use std::collections::{HashMap, HashSet};

use ruff_python_trivia::{CommentLinePosition, CommentRanges, SuppressionKind};
use ruff_source_file::{LineRanges, OneIndexed, SourceCode};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::rule::RuleId;

/// One line's parsed `# prose: ignore` or `# prose: skip[<id>]`
/// directive.
#[derive(Debug)]
enum RuleEntry {
    /// Bare `# prose: ignore`. Suppresses every rule on the line.
    All,
    /// `# prose: ignore[<id>[, <id>...]]` or `# prose: skip[<id>[,
    /// <id>...]]`. Unknown ids are dropped.
    Specific(HashSet<RuleId>),
}

impl RuleEntry {
    /// Returns `true` when `self` suppresses `rule`. `All` matches
    /// every id, `Specific` matches only listed ids.
    fn matches(&self, rule: RuleId) -> bool {
        match self {
            Self::All => true,
            Self::Specific(rules) => rules.contains(&rule),
        }
    }

    /// Folds `incoming` into `self`. `All` widens any prior `Specific`,
    /// and a second `Specific` unions its ids into the first.
    fn merge(&mut self, incoming: Self) {
        match (&mut *self, incoming) {
            (Self::All, _) => {}
            (slot @ Self::Specific(_), Self::All) => *slot = Self::All,
            (Self::Specific(rules), Self::Specific(more)) => rules.extend(more),
        }
    }
}

impl Default for RuleEntry {
    fn default() -> Self {
        Self::Specific(HashSet::new())
    }
}

/// Sorted byte-range list for format-suppression spans paired with two
/// per-line `OneIndexed` maps. `lints` carries `# prose: ignore` per-line
/// lint directives, `skips` carries `# prose: skip[<id>]` per-rule format
/// directives. Span queries run in O(log n), per-line queries in O(1).
#[derive(Debug)]
pub(crate) struct SuppressionMap {
    file_suppressed: bool,
    lints: HashMap<OneIndexed, RuleEntry>,
    skips: HashMap<OneIndexed, RuleEntry>,
    spans: Vec<TextRange>,
}

impl SuppressionMap {
    /// Walks `comments` against `source`, classifying each comment via
    /// `classify_format_directive` for the format spans and per-line
    /// skip index, and `find_prose_ignore` for the lint index.
    /// `first_code_offset` carries the start of the source's first
    /// top-level statement (or `None` for code-free input), powering
    /// the `file_is_suppressed` shortcut.
    ///
    /// An unmatched `# prose: off` (or alias) extends through end of
    /// file, a stray `# prose: on` is a no-op, and two consecutive
    /// `# prose: off` directives flatten with the first `# prose: on`
    /// closing the block. Multiple `# prose: ignore` directives on the
    /// same line merge with bare-wins precedence, and `# prose: skip[<id>]`
    /// directives union their listed ids.
    pub(crate) fn from_comments(
        source: &SourceCode<'_, '_>,
        comments: &CommentRanges,
        first_code_offset: Option<TextSize>,
    ) -> Self {
        let source_text = source.text();
        let mut lints: HashMap<OneIndexed, RuleEntry> = HashMap::new();
        let mut skips: HashMap<OneIndexed, RuleEntry> = HashMap::new();
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
                        spans.push(source_text.full_line_range(range.start()));
                    }
                    FormatDirective::SkipRules(rules) => {
                        let line = source.line_index(range.start());
                        skips
                            .entry(line)
                            .or_default()
                            .merge(RuleEntry::Specific(rules));
                    }
                    _ => {}
                }
            }
            if let Some(entry) = find_prose_ignore(comment) {
                let line = source.line_index(range.start());
                lints.entry(line).or_default().merge(entry);
            }
        }
        let file_suppressed =
            open_off.is_some_and(|off| first_code_offset.is_none_or(|code| off <= code));
        spans.extend(open_off.map(|start| TextRange::new(start, source_text.text_len())));
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
    /// format-suppression span.
    pub(crate) fn has_format_suppression(&self) -> bool {
        !self.spans.is_empty()
    }

    /// Returns `true` when the source carries at least one
    /// `# prose: ignore` directive.
    pub(crate) fn has_lint_suppression(&self) -> bool {
        !self.lints.is_empty()
    }

    /// Returns `true` when the source carries at least one
    /// `# prose: skip[<id>]` directive.
    pub(crate) fn has_skip_suppression(&self) -> bool {
        !self.skips.is_empty()
    }

    /// Returns `true` when `ranged`'s span overlaps any
    /// format-suppressed span by at least one byte. Empty ranges
    /// report overlap when their offset strictly sits inside a span.
    pub(crate) fn intersects<R: Ranged>(&self, ranged: R) -> bool {
        self.spans
            .binary_search_by(|s| s.ordering(ranged.range()))
            .is_ok()
    }

    /// Returns `true` when `line` carries a `# prose: skip[<id>]`
    /// directive that lists `rule`.
    pub(crate) fn is_format_suppressed_at(&self, line: OneIndexed, rule: RuleId) -> bool {
        self.skips.get(&line).is_some_and(|e| e.matches(rule))
    }

    /// Returns `true` when `line` carries a `# prose: ignore`
    /// directive that suppresses `rule`. Bare directives suppress
    /// every rule on their line.
    pub(crate) fn is_lint_suppressed_at(&self, line: OneIndexed, rule: RuleId) -> bool {
        self.lints.get(&line).is_some_and(|e| e.matches(rule))
    }
}

/// Result of `classify_format_directive`. `Kind` carries an upstream
/// or `# prose:`-prefixed off/on/skip directive that drives the span
/// machinery, whereas `SkipRules` carries the rule-id list parsed from
/// `# prose: skip[<id>[, <id>...]]`.
enum FormatDirective {
    Kind(SuppressionKind),
    SkipRules(HashSet<RuleId>),
}

/// Strips the leading `prose:` marker from `after_hash` and returns
/// the trimmed body. Returns `None` for any other shape.
fn after_prose_prefix(after_hash: &str) -> Option<&str> {
    after_hash.trim().strip_prefix("prose:").map(str::trim)
}

/// Classifies `comment` against the three format-suppression
/// namespaces. The `# prose:` namespace is tried first, falling
/// through to `SuppressionKind::from_comment` for the `# fmt:` and
/// `# yapf:` aliases. `# prose: skip[<id>...]` returns `SkipRules`,
/// `# prose: off|on|skip` and the alias forms return `Kind`. Multiple
/// `# prose: skip[<id>]` chunks on one comment range union their ids.
fn classify_format_directive(comment: &str) -> Option<FormatDirective> {
    comment
        .split('#')
        .skip(1)
        .filter_map(parse_prose_format)
        .reduce(|mut acc, next| {
            if let (FormatDirective::SkipRules(a), FormatDirective::SkipRules(b)) = (&mut acc, next)
            {
                a.extend(b);
            }
            acc
        })
        .or_else(|| SuppressionKind::from_comment(comment).map(FormatDirective::Kind))
}

/// Splits `comment` at each `#` boundary, parsing each chunk as a
/// `prose: ignore` directive and folding successful hits through
/// `RuleEntry::merge`. Catches nested forms like `# my note # prose:
/// ignore` and multi-directive lines like `# prose: ignore  # prose:
/// ignore[align-equals]`.
fn find_prose_ignore(comment: &str) -> Option<RuleEntry> {
    comment
        .split('#')
        .skip(1)
        .filter_map(parse_prose_ignore)
        .reduce(|mut acc, next| {
            acc.merge(next);
            acc
        })
}

fn merge_spans(mut spans: Vec<TextRange>) -> Vec<TextRange> {
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

/// Parses the rule-id body of a `[<id>[, <id>...]]` suffix into a
/// `RuleEntry::Specific`. Returns `None` when the brackets are missing
/// or malformed. Unknown rule ids are silently dropped.
fn parse_bracketed_rule_list(body: &str) -> Option<HashSet<RuleId>> {
    Some(
        body.strip_prefix('[')?
            .strip_suffix(']')?
            .split(',')
            .filter_map(|part| part.trim().parse::<RuleId>().ok())
            .collect(),
    )
}

/// Parses the post-`#` body of a `prose: off`, `prose: on`, `prose:
/// skip`, or `prose: skip[<id>...]` directive. Returns `None` for any
/// other shape, leaving the caller to try the `# fmt:` / `# yapf:`
/// fallback.
fn parse_prose_format(after_hash: &str) -> Option<FormatDirective> {
    let body = after_prose_prefix(after_hash)?;
    if let Some(rest) = body.strip_prefix("skip").map(str::trim) {
        if rest.is_empty() {
            return Some(FormatDirective::Kind(SuppressionKind::Skip));
        }
        return parse_bracketed_rule_list(rest).map(FormatDirective::SkipRules);
    }
    match body {
        "off" => Some(FormatDirective::Kind(SuppressionKind::Off)),
        "on" => Some(FormatDirective::Kind(SuppressionKind::On)),
        _ => None,
    }
}

/// Parses the post-`#` body of a `prose: ignore`, `prose: ignore[<id>]`,
/// or `prose: ignore[<id>, <id>...]` directive. Returns `None` for any
/// other shape. Whitespace tolerated around `:`, `[`, `,`, and `]`.
/// Unknown rule ids inside the brackets are dropped.
fn parse_prose_ignore(after_hash: &str) -> Option<RuleEntry> {
    let body = after_prose_prefix(after_hash)?
        .strip_prefix("ignore")?
        .trim();
    if body.is_empty() {
        return Some(RuleEntry::All);
    }
    parse_bracketed_rule_list(body).map(RuleEntry::Specific)
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
    fn bare_prose_skip_opens_a_full_line_span() {
        let source = parse("x = 1  # prose: skip\n");
        let map = source.suppression_map();
        assert!(map.has_format_suppression());
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
        assert!(!map.has_format_suppression());
        assert!(!map.has_lint_suppression());
        assert!(!map.has_skip_suppression());
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
        assert!(!map.has_skip_suppression());
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
            "x = 1  # prose: skip[align-equals\n",
            "x = 1  # prose: skip extra\n",
        ] {
            let source = parse(src);
            let map = source.suppression_map();
            assert!(
                !map.has_lint_suppression(),
                "expected no lint suppression for {src:?}",
            );
            assert!(
                !map.has_skip_suppression(),
                "expected no skip suppression for {src:?}",
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
    fn multiple_skip_directives_on_one_comment_union_their_rules() {
        let source = parse("x = 1  # prose: skip[align-equals]  # prose: skip[alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.is_format_suppressed_at(line(0), align_equals()));
        assert!(map.is_format_suppressed_at(line(0), alphabetize()));
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
        assert!(source
            .suppression_map()
            .intersects(range(x_offset, x_offset + 5)),);
    }

    #[test]
    fn prose_off_and_fmt_off_open_the_same_span() {
        for text in [
            "# prose: off\nx = 1\n# prose: on\n",
            "# fmt: off\nx = 1\n# fmt: on\n",
            "# prose: off\nx = 1\n",
            "# fmt: off\nx = 1\n",
        ] {
            let src = parse(text);
            let x_offset = src.text().find('x').expect("x is present") as u32;
            assert!(
                src.suppression_map()
                    .intersects(range(x_offset, x_offset + 5)),
                "expected suppression to cover `x = 1` in {text:?}",
            );
        }
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
    fn skip_brackets_target_only_listed_rules() {
        let source = parse("x = 1  # prose: skip[align-equals]\n");
        let map = source.suppression_map();
        assert!(map.has_skip_suppression());
        assert!(map.is_format_suppressed_at(line(0), align_equals()));
        assert!(!map.is_format_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn skip_multi_id_suppresses_each_listed_rule() {
        let source = parse("x = 1  # prose: skip[align-equals, alphabetize]\n");
        let map = source.suppression_map();
        assert!(map.is_format_suppressed_at(line(0), align_equals()));
        assert!(map.is_format_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn skip_unknown_id_is_dropped_silently() {
        let source = parse("x = 1  # prose: skip[align-equals, not-a-rule]\n");
        let map = source.suppression_map();
        assert!(map.is_format_suppressed_at(line(0), align_equals()));
        assert!(!map.is_format_suppressed_at(line(0), alphabetize()));
    }

    #[test]
    fn skip_whitespace_tolerant_inside_brackets() {
        let canonical = parse("x = 1  # prose: skip[align-equals, alphabetize]\n");
        let compact = parse("x = 1  # prose:skip[ align-equals ,alphabetize ]\n");
        let canonical_map = canonical.suppression_map();
        let compact_map = compact.suppression_map();
        for rule in [align_equals(), alphabetize()] {
            assert_eq!(
                canonical_map.is_format_suppressed_at(line(0), rule),
                compact_map.is_format_suppressed_at(line(0), rule),
            );
            assert!(canonical_map.is_format_suppressed_at(line(0), rule));
        }
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

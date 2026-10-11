//! Per-`Source` index of `# prose: off` / `# prose: on` / `# prose: skip`
//! spans (plus the `# fmt:` and `# yapf:` aliases), `# prose: skip[<id>]`
//! per-rule format directives, and `# prose: ignore[<id>]` per-line lint
//! directives. Built once during `Source` construction, consulted by
//! `Pipeline::run` to drop suppressed fix groups and `Severity::Lint`
//! diagnostics, and read by the `inert-directives` audit for the
//! directives themselves. A skip directive that closes its logical
//! line spans every physical line that line occupies and suppresses
//! rewrites alone, leaving lint diagnostics to the `ignore` directives,
//! whereas an off region suppresses both. A `file_is_suppressed`
//! shortcut lets the pipeline skip rule execution entirely when an
//! unmatched off precedes every statement.

use itertools::Itertools;
use ruff_notebook::CellOffsets;
use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_python_trivia::{CommentLinePosition, CommentRanges, SuppressionKind};
use ruff_source_file::{LineRanges, OneIndexed, SourceCode};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::{
    primitives::range::{merged_spans, overlaps},
    rules::RuleId,
};

mod directive;
mod parse;

pub(crate) use directive::{Directive, Named, RuleEntry};
use parse::directives;
pub(crate) use parse::{is_directive_comment, is_keep_marker};

/// Sorted byte-range lists for the `# prose: off` regions and the bare
/// `# prose: skip` spans, beside every skip directive with the span it
/// suppresses and every `# prose: ignore` directive with the line it
/// sits on. An off region suppresses rewrites and lint diagnostics
/// alike, whereas a skip span suppresses rewrites alone and leaves lints
/// to the `ignore` directives. `closes` holds the line start of each
/// `# prose: on` that closes a region, and `listed` the position in
/// `skips` of each skip naming its rules. Span queries run in O(log n)
/// against `spans` and `skip_spans`, O(n) against the listed skips and
/// `closes`, and O(log n) per line against `ignores`.
#[derive(Clone, Debug)]
pub(crate) struct SuppressionMap {
    closes: Vec<TextSize>,
    file_suppressed: bool,
    ignores: Vec<(OneIndexed, Directive)>,
    listed: Vec<usize>,
    skip_spans: Vec<TextRange>,
    skips: Vec<(TextRange, Directive)>,
    spans: Vec<TextRange>,
}

impl SuppressionMap {
    /// Walks `comments` against `source`, indexing the off regions, the
    /// skip directives with their spans, and the per-line lint
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
        let mut closes: Vec<TextSize> = Vec::new();
        let mut ignores: Vec<(OneIndexed, Directive)> = Vec::new();
        let mut skips: Vec<(TextRange, Directive)> = Vec::new();
        let mut spans: Vec<TextRange> = Vec::new();
        let mut open_off: Option<TextSize> = None;
        for range in comments {
            let comment = &source_text[range];
            let found = directives(comment);
            if let Some(kind) = found.region
                && CommentLinePosition::for_range(range, source_text).is_own_line()
            {
                match kind {
                    SuppressionKind::Off => {
                        open_off.get_or_insert_with(|| source_text.line_start(range.start()));
                    }
                    SuppressionKind::On => {
                        if let Some(start) = open_off.take() {
                            let close = source_text.line_start(range.start());
                            closes.push(close);
                            spans.push(TextRange::new(start, close));
                        }
                    }
                    SuppressionKind::Skip => {}
                }
            }
            let at_comment = |named| Directive {
                comment: range,
                named,
            };
            if let Some(named) = found.skip {
                skips.push((skip_span(source_text, tokens, range), at_comment(named)));
            }
            if let Some(named) = found.ignore {
                ignores.push((source.line_index(range.start()), at_comment(named)));
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
            closes,
            file_suppressed,
            ignores,
            listed: Vec::new(),
            skip_spans: Vec::new(),
            skips,
            spans: merged_spans(spans),
        }
        .indexed()
    }

    /// This map with `listed` and `skip_spans` derived from its skips.
    fn indexed(mut self) -> Self {
        self.listed = self
            .skips
            .iter()
            .positions(|(_, skip)| matches!(skip.named.entry, RuleEntry::Specific(_)))
            .collect();
        self.skip_spans = merged_spans(
            self.skips
                .iter()
                .filter(|(_, skip)| matches!(skip.named.entry, RuleEntry::All))
                .map(|&(span, _)| span)
                .collect(),
        );
        self
    }

    /// Returns `true` when an unmatched `# prose: off` (or alias) sits
    /// at or before the first non-blank, non-comment line of the file.
    pub(crate) fn file_is_suppressed(&self) -> bool {
        self.file_suppressed
    }

    /// Returns `true` when the source carries at least one off region,
    /// bare skip span, or `# prose: skip[<id>]` directive.
    pub(crate) fn has_format_suppression(&self) -> bool {
        !self.spans.is_empty() || !self.skips.is_empty()
    }

    /// Returns `true` when the source carries at least one
    /// `# prose: ignore` directive.
    pub(crate) fn has_lint_suppression(&self) -> bool {
        !self.ignores.is_empty()
    }

    /// The `# prose: ignore` directive on `line`, `None` where the line
    /// carries none.
    pub(crate) fn ignore_at(&self, line: OneIndexed) -> Option<&Directive> {
        self.ignores
            .binary_search_by_key(&line, |&(at, _)| at)
            .ok()
            .map(|index| &self.ignores[index].1)
    }

    /// Every `# prose: ignore` directive beside the line it sits on, in
    /// source order.
    pub(crate) fn ignores(&self) -> &[(OneIndexed, Directive)] {
        &self.ignores
    }

    /// Returns `true` when `ranged`'s span overlaps a `# prose: off`
    /// region by at least one byte. Empty ranges report overlap when
    /// their offset strictly sits inside a region. A bare
    /// `# prose: skip` opens no region, so it does not report here.
    pub(crate) fn intersects<R: Ranged>(&self, ranged: R) -> bool {
        overlaps(ranged.range(), &self.spans)
    }

    /// Returns `true` when `line` carries a `# prose: ignore`
    /// directive that suppresses `rule`. Bare directives suppress
    /// every rule on their line.
    pub(crate) fn is_lint_suppressed_at(&self, line: OneIndexed, rule: RuleId) -> bool {
        self.ignore_at(line)
            .is_some_and(|ignore| ignore.named.entry.matches(rule))
    }

    /// Returns `true` when a reorder by `rule` leaves `ranged` in place,
    /// meaning `ranged` overlaps a span [`Self::suppresses`] reports for
    /// `rule` or holds the line of a `# prose: on` that closes an off
    /// region.
    pub(crate) fn pins<R: Ranged>(&self, ranged: R, rule: RuleId) -> bool {
        let range = ranged.range();
        self.suppresses(range, rule) || self.closes.iter().any(|&close| range.contains(close))
    }

    /// Returns `true` when the span of a skip directive, whatever rules
    /// it names, overlaps `ranged`.
    pub(crate) fn skipped<R: Ranged>(&self, ranged: R) -> bool {
        let range = ranged.range();
        self.skips
            .iter()
            .any(|(span, _)| span.ordering(range).is_eq())
    }

    /// Every skip directive beside the span it suppresses, in source
    /// order.
    pub(crate) fn skips(&self) -> &[(TextRange, Directive)] {
        &self.skips
    }

    /// Returns `true` when `ranged` overlaps a `# prose: off` region, a
    /// bare `# prose: skip` span, or a `# prose: skip[<id>]` span
    /// listing `rule`.
    pub(crate) fn suppresses<R: Ranged>(&self, ranged: R, rule: RuleId) -> bool {
        let range = ranged.range();
        overlaps(range, &self.spans)
            || overlaps(range, &self.skip_spans)
            || self.listed.iter().any(|&index| {
                let (span, skip) = &self.skips[index];
                span.ordering(range).is_eq() && skip.named.entry.matches(rule)
            })
    }

    /// This map with the skip directive at `index` of [`Self::skips`]
    /// removed, every other directive and region left as it stands.
    pub(crate) fn without_skip(&self, index: usize) -> Self {
        let mut map = self.clone();
        map.skips.remove(index);
        map.indexed()
    }
}

/// The offset an unmatched `# prose: off` opened at `start` closes at,
/// the end of the notebook cell holding `start` or the buffer's end for
/// an ordinary module whose `cell_offsets` are empty.
fn cell_close_end(cell_offsets: &CellOffsets, source_text: &str, start: TextSize) -> TextSize {
    cell_offsets
        .containing_range(start)
        .map_or(source_text.text_len(), TextRange::end)
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
mod tests;

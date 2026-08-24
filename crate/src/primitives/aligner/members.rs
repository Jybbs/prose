//! `Member` construction for the alignment rules: builders that anchor
//! a row's aligned token and measure its display width, over the gap
//! locator those builders and the comment rules share.

use ruff_python_ast::{
    AnyParameterRef, Parameters,
    token::{Token, TokenKind},
};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{TextLen, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::Member;
use crate::{primitives::padding::delimiter_padding_gaps, source::Source};

/// Builds a `Member` for a row whose aligned token sits at `anchor`.
/// Width is the display width of the line's content from the first
/// non-whitespace character to the last non-whitespace character
/// before the gap, leaving the gap free for the rule to rewrite, the
/// baseline is the indent ahead of that content, and the settled width
/// reads past the delimiter padding `strip-stranded-padding` deletes.
pub(crate) fn line_anchored_member(source: &Source, anchor: TextSize) -> Member {
    let line_start = source.text().line_start(anchor);
    let gap = line_gap_before(source, anchor);
    let head = TextRange::new(line_start, gap.start());
    let text = source.slice(head);
    let width = text.trim_whitespace_start().width();
    let padding: usize = delimiter_padding_gaps(source, head)
        .map(|gap| source.slice(gap).width())
        .sum();
    Member {
        baseline: text.width() - width,
        gap,
        line_start,
        op_width: 0,
        settled_width: width - padding,
        value_gap: None,
        width,
    }
}

/// Builds a `Member` whose anchor is the first `kind` token in `search`
/// [confined to one line](single_line_anchor) with `lhs_start`, so a
/// left-hand side broken across lines stays unaligned.
pub(crate) fn line_anchored_member_at_kind(
    source: &Source,
    lhs_start: TextSize,
    search: TextRange,
    kind: TokenKind,
) -> Option<Member> {
    single_line_anchor(source, lhs_start, search, |t| t.kind() == kind)
        .map(|anchor| line_anchored_member(source, anchor))
}

/// Builds a `Member` anchored on the first `kind` token between
/// `lhs.end()` and `rhs_start`, [confined to one line](single_line_anchor)
/// with `lhs.start()`, so a left-hand side broken across lines stays
/// unaligned. The scan opens past `lhs.end()`, so a `kind` token inside
/// the left-hand side never anchors.
pub(crate) fn line_anchored_member_between(
    source: &Source,
    lhs: TextRange,
    rhs_start: TextSize,
    kind: TokenKind,
) -> Option<Member> {
    line_anchored_member_at_kind(
        source,
        lhs.start(),
        TextRange::new(lhs.end(), rhs_start),
        kind,
    )
}

/// The whitespace run ending at `anchor`, opening no earlier than the
/// start of `anchor`'s line.
pub(crate) fn line_gap_before(source: &Source, anchor: TextSize) -> TextRange {
    let line_start = source.text().line_start(anchor);
    let trimmed = source
        .slice(TextRange::new(line_start, anchor))
        .trim_whitespace_end();
    TextRange::new(line_start + trimmed.text_len(), anchor)
}

/// Walks `params` in source order, qualifying each parameter through
/// `qualify` and returning one group per run of contiguous qualified
/// parameters. A parameter that fails to qualify breaks the current
/// run without joining either neighbor. Empty runs are filtered out.
pub(crate) fn parameter_split_groups<F>(params: &Parameters, qualify: F) -> Vec<Vec<Member>>
where
    F: FnMut(AnyParameterRef<'_>) -> Option<Member>,
{
    let qualified: Vec<_> = params.iter_source_order().map(qualify).collect();
    qualified
        .split(Option::is_none)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| chunk.iter().copied().flatten().collect())
        .collect()
}

/// Builds a `Member` whose anchor is the first `search` token satisfying
/// `predicate` and [confined to one line](single_line_anchor) with
/// `target.start()`, measuring width by `target` plus `extra_width`.
pub(crate) fn range_anchored_member_single_line<F>(
    source: &Source,
    target: TextRange,
    search: TextRange,
    predicate: F,
    extra_width: usize,
) -> Option<Member>
where
    F: FnMut(&Token) -> bool,
{
    single_line_anchor(source, target.start(), search, predicate)
        .map(|anchor| range_anchored_member(source, target, anchor, extra_width))
}

/// Builds a `Member` for a row whose aligned token sits at `anchor`,
/// with width measured by the display width of `target` plus
/// `extra_width` and the baseline at `target`'s own column. Pass
/// `extra_width = 0` when the LHS is exactly `target` (e.g. `x = 1`),
/// and pass a non-zero value when the LHS visually extends past
/// `target` by characters not covered by the slice (e.g. the `+` of
/// `x += 1` widens the LHS by one column without being part of the
/// target range).
fn range_anchored_member(
    source: &Source,
    target: TextRange,
    anchor: TextSize,
    extra_width: usize,
) -> Member {
    let width = source.slice(target).width() + extra_width;
    let line_start = source.text().line_start(anchor);
    Member {
        baseline: source.width_between(line_start, target.start()),
        gap: TextRange::new(target.end(), anchor),
        line_start,
        op_width: 0,
        settled_width: width - delimiter_padding_width(source, target),
        value_gap: None,
        width,
    }
}

/// Returns the offset of the first token in `search` satisfying
/// `predicate`, or `None` when none matches or the span from
/// `guard_start` to that token crosses a line break. A member measures
/// its width from the anchor's own line, so a cross-line anchor would
/// align against the wrong line and is held out.
fn single_line_anchor<F>(
    source: &Source,
    guard_start: TextSize,
    search: TextRange,
    predicate: F,
) -> Option<TextSize>
where
    F: FnMut(&Token) -> bool,
{
    let anchor = source.first_token_offset_in_range(search, predicate)?;
    source.same_line(guard_start, anchor).then_some(anchor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::parse;

    #[test]
    fn line_anchored_member_at_kind_admits_same_line_anchor() {
        // Key, colon, and value share one line, so the member builds.
        let source = parse("{k: v}\n");
        let member = line_anchored_member_at_kind(
            &source,
            TextSize::new(1),
            TextRange::new(TextSize::new(2), TextSize::new(4)),
            TokenKind::Colon,
        );
        assert!(member.is_some());
    }

    #[test]
    fn line_anchored_member_at_kind_rejects_cross_line_anchor() {
        // The `:` opens the line after the key, so the span from the
        // key's start to the anchor crosses a break and nothing builds.
        let source = parse("{\n    k\n    : v,\n}\n");
        let member = line_anchored_member_at_kind(
            &source,
            TextSize::new(6),
            TextRange::new(TextSize::new(7), TextSize::new(14)),
            TokenKind::Colon,
        );
        assert!(member.is_none());
    }

    #[test]
    fn line_anchored_member_collapses_gap_at_line_start() {
        let source = parse("xy\n");
        let member = line_anchored_member(&source, TextSize::new(0));

        // anchor sits at line start, with empty gap and zero width.
        assert_eq!(member.gap.start(), member.gap.end());
        assert_eq!(member.width, 0);
    }
}

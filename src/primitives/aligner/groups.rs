//! Statement grouping and member construction for the alignment
//! rules. Walks a body into line-adjacent runs of qualified members,
//! relaxing adjacency across a skip-held row so its neighbors align as
//! one block.

use ruff_python_ast::{
    AnyParameterRef, Parameters, Stmt,
    token::{Token, TokenKind},
};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::Member;
use crate::{rule::RuleId, source::Source};

/// Moves the in-progress run into `groups` when it holds at least one
/// member, leaving `current` empty for the next run.
pub(crate) fn flush_run<M>(groups: &mut Vec<Vec<M>>, current: &mut Vec<M>) {
    if !current.is_empty() {
        groups.push(std::mem::take(current));
    }
}

/// Returns `true` when `members` form a multi-row group whose
/// aligned tokens sit on distinct source lines.
pub(crate) fn is_alignment_candidate(members: &[Member]) -> bool {
    members.len() >= 2
        && members
            .windows(2)
            .all(|w| w[0].line_start != w[1].line_start)
}

/// Returns `true` when the line containing `anchor` carries a skip
/// directive for `rule`: a bare `# prose: skip` / `# fmt: skip` span, a
/// `# fmt: off` region, or `# prose: skip[rule]`. A held row stays out
/// of the column math and emits no edit, so its neighbors align around
/// it. Short-circuits when the source carries no format suppression.
pub(crate) fn is_held(source: &Source, rule: RuleId, anchor: TextSize) -> bool {
    let suppression = source.suppression_map();
    if !suppression.has_format_suppression() && !suppression.has_skip_suppression() {
        return false;
    }
    suppression.intersects(source.text().full_line_range(anchor))
        || suppression.is_format_suppressed_at(source.line_index(anchor), rule)
}

/// Generalization of [`line_adjacent_groups`] for rules that admit
/// more than one member shape. The qualifier returns `Option<(K, M)>`
/// where `K` tags the shape, and a run extends only while the next
/// member shares the active key, sits line-adjacent to the prior
/// statement, and the prior statement itself fits on one source line.
/// A key change at an otherwise-adjacent boundary closes the active
/// run and starts a fresh one without losing the boundary statement.
/// A statement [held](is_held) for `rule` is transparent: it joins no
/// group and does not close the run, leaving neighbors on either side
/// to align as one block. Adjacency across a held statement relaxes to
/// a consecutive-line check, so the held row's own trailing skip
/// comment does not break the run while a standalone comment or blank
/// line between rows still does. Walks `body` exactly once.
pub(crate) fn keyed_line_adjacent_groups<'a, K, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    rule: RuleId,
    mut qualify: F,
) -> Vec<Vec<M>>
where
    K: Eq,
    F: FnMut(&'a Stmt) -> Option<(K, M)>,
{
    let mut groups: Vec<Vec<M>> = Vec::new();
    let mut current: Vec<M> = Vec::new();
    let mut active: Option<(K, TextRange, bool)> = None;
    for stmt in body {
        let Some((key, member)) = qualify(stmt) else {
            flush_run(&mut groups, &mut current);
            active = None;
            continue;
        };
        if is_held(source, rule, stmt.start()) {
            if let Some((_, prev, prev_held)) = active.as_mut() {
                *prev = stmt.range();
                *prev_held = true;
            }
            continue;
        }
        let extends = active
            .as_ref()
            .is_some_and(|(active_key, prev, prev_held)| {
                active_key == &key
                    && !source.contains_line_break(prev)
                    && run_continues(source, prev.end(), *prev_held, stmt.start())
            });
        if !extends {
            flush_run(&mut groups, &mut current);
        }
        current.push(member);
        active = Some((key, stmt.range(), false));
    }
    flush_run(&mut groups, &mut current);
    groups
}

/// Walks `body`, qualifying each statement through `qualify` and
/// grouping the qualified members into runs where every consecutive
/// pair sits on adjacent source lines. A multi-line prior statement,
/// a non-qualifying statement, a comment in the inter-statement gap,
/// or a blank line breaks the current run. A statement held for `rule`
/// is transparent per [`keyed_line_adjacent_groups`]. Empty groups
/// (statements that fail qualification with no qualified neighbors) are
/// skipped. Thin wrapper over [`keyed_line_adjacent_groups`] for rules
/// whose qualifier produces only one form, so every member shares an
/// implicit `()` key.
pub(crate) fn line_adjacent_groups<'a, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    rule: RuleId,
    mut qualify: F,
) -> Vec<Vec<M>>
where
    F: FnMut(&'a Stmt) -> Option<M>,
{
    keyed_line_adjacent_groups(source, body, rule, move |stmt| {
        qualify(stmt).map(|m| ((), m))
    })
}

/// Builds a `Member` for a row whose aligned token sits at `anchor`.
/// Width is the display width of the line's content from the first
/// non-whitespace character to the last non-whitespace character
/// before the gap, leaving the gap free for the rule to rewrite.
pub(crate) fn line_anchored_member(source: &Source, anchor: TextSize) -> Member {
    let line_start = source.text().line_start(anchor);
    let prefix = source.slice(TextRange::new(line_start, anchor));
    let trimmed_end = prefix.trim_whitespace_end();
    let gap_start = line_start + TextSize::of(trimmed_end);
    Member {
        gap: TextRange::new(gap_start, anchor),
        line_start,
        op_width: 0,
        width: trimmed_end.trim_whitespace_start().width(),
    }
}

/// Builds a `Member` whose anchor is the first token of `kind` within
/// `search`. Returns `None` when the search turns up nothing.
pub(crate) fn line_anchored_member_at_kind(
    source: &Source,
    search: TextRange,
    kind: TokenKind,
) -> Option<Member> {
    let anchor = source.first_token_offset_in_range(search, |t| t.kind() == kind)?;
    Some(line_anchored_member(source, anchor))
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

/// Builds a `Member` whose anchor is the first token in `search`
/// satisfying `predicate`, with width measured by `target` plus
/// `extra_width`. Returns `None` if no token matches, or if the span
/// from `target.start()` to the anchor crosses a newline (continuation
/// imports, line-broken assignments). Use this when alignment must
/// stay confined to a single source line.
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
    let anchor = source.first_token_offset_in_range(search, predicate)?;
    if source.contains_line_break(TextRange::new(target.start(), anchor)) {
        return None;
    }
    Some(range_anchored_member(source, target, anchor, extra_width))
}

/// Builds a `Member` for a row whose aligned token sits at `anchor`,
/// with width measured by the display width of `target` plus
/// `extra_width`. Pass `extra_width = 0` when the LHS is exactly
/// `target` (e.g. `x = 1`), and pass a non-zero value when the LHS
/// visually extends past `target` by characters not covered by the
/// slice (e.g. the `+` of `x += 1` widens the LHS by one column
/// without being part of the target range).
fn range_anchored_member(
    source: &Source,
    target: TextRange,
    anchor: TextSize,
    extra_width: usize,
) -> Member {
    Member {
        gap: TextRange::new(target.end(), anchor),
        line_start: source.text().line_start(anchor),
        op_width: 0,
        width: source.slice(target).width() + extra_width,
    }
}

/// Returns whether a run continues from a row ending at `prev_end` to
/// the next row starting at `next_start`. A non-held predecessor uses
/// the standard inter-statement adjacency. A [held](is_held)
/// predecessor relaxes to a consecutive-line check, so the held row's
/// own trailing skip comment does not break the run while a standalone
/// comment or blank line between rows still does.
fn run_continues(
    source: &Source,
    prev_end: TextSize,
    prev_held: bool,
    next_start: TextSize,
) -> bool {
    if prev_held {
        source.consecutive_lines(prev_end, next_start)
    } else {
        source.is_line_adjacent(TextRange::new(prev_end, next_start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    #[test]
    fn keyed_line_adjacent_groups_breaks_on_blank_line_after_held() {
        let source = parse("x = 1\ny = 2  # prose: skip[align-equals]\n\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_breaks_on_standalone_comment_after_held() {
        let source = parse("x = 1\ny = 2  # prose: skip[align-equals]\n# note\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        // A standalone comment after the held line is not consecutive,
        // so the relaxed adjacency still breaks the run.
        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_flushes_trailing_active_run() {
        let source = parse("x = 1\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_holds_member_with_extra_comment_on_its_line() {
        let source = parse("x = 1\ny = 2  # note  # prose: skip[align-equals]\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        // The held line's extra trailing comment rides along with it,
        // so x and z still bridge across it into one run.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_holds_skip_suppressed_member_and_bridges_run() {
        let source = parse("x = 1\ny = 2  # prose: skip[align-equals]\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        // y is held, so it joins no group, yet x and z bridge across it
        // into one run.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_merges_same_key_adjacent_stmts() {
        let source = parse("x = 1\ny = 2\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn keyed_line_adjacent_groups_non_qualifier_closes_active_run() {
        let source = parse("x = 1\npass\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_returns_empty_for_empty_body() {
        let source = parse("");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert!(groups.is_empty());
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_blank_line() {
        let source = parse("x = 1\n\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_comment_in_gap() {
        let source = parse("x = 1\n# comment\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_key_change_at_adjacent_boundary() {
        // Two assigns flanking an aug-assign, all line-adjacent. The
        // distinct keys force the run to split even though no whitespace
        // breaks the adjacency, exercising the `keyed`-only invariant.
        let source = parse("x = 1\ny += 2\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| {
                if s.is_assign_stmt() {
                    Some(("assign", ()))
                } else if s.is_aug_assign_stmt() {
                    Some(("aug", ()))
                } else {
                    None
                }
            },
        );

        assert_eq!(
            groups.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![1, 1, 1],
        );
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_multiline_prior_stmt() {
        let source = parse("x = {\n    'a': 1,\n}\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_yields_singleton_for_lone_qualifier() {
        let source = parse("x = 1\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
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

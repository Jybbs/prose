//! Statement grouping for the alignment rules. Walks a body into
//! line-adjacent runs of qualified members, relaxing adjacency across a
//! skip-held row so its neighbors align as one block.

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::holds::is_held;
use crate::{rule::RuleId, source::Source};

/// The disposition of one item walked by [`adjacent_member_groups`].
pub(crate) enum Slot<M> {
    /// A row that ends the active run without joining either side, the
    /// way an undefaulted parameter ends a parameter run.
    Break,
    /// A passthrough row that bridges the run without joining, the way a
    /// `**spread` dict entry passes alignment through its neighbors.
    Bridge,
    /// A qualifying row that joins the active run.
    Member(M),
}

/// A qualified row converts to [`Slot::Member`] and an unqualified row
/// to [`Slot::Break`], the classification for a run with no passthrough
/// rows.
impl<M> From<Option<M>> for Slot<M> {
    fn from(member: Option<M>) -> Self {
        member.map_or(Self::Break, Self::Member)
    }
}

/// Walks `items` in source order, classifying each through `classify`
/// and gathering members into runs whose consecutive members sit on
/// directly adjacent source lines. A `Bridge` extends the run's anchor
/// without joining, a `Break` closes the run, and a standalone comment
/// or blank line between two members closes it as well. When
/// `break_after_multiline` is set, a member whose own range spans more
/// than one line also closes the run after it, so a run never extends
/// past a multi-line row. The multi-line flag tracks the last member
/// only, so a `Bridge` extends the run's reach without tripping the
/// break for a held row.
pub(crate) fn adjacent_member_groups<T, M, F>(
    source: &Source,
    items: impl IntoIterator<Item = T>,
    break_after_multiline: bool,
    mut classify: F,
) -> Vec<Vec<M>>
where
    T: Ranged,
    F: FnMut(T) -> Slot<M>,
{
    let mut groups: Vec<Vec<M>> = Vec::new();
    let mut current: Vec<M> = Vec::new();
    let mut prev_end: Option<TextSize> = None;
    let mut prev_multiline = false;
    for item in items {
        let range = item.range();
        match classify(item) {
            Slot::Member(member) => {
                let extends = prev_end.is_some_and(|end| {
                    source.consecutive_lines(end, range.start())
                        && !(break_after_multiline && prev_multiline)
                });
                if !extends {
                    flush_run(&mut groups, &mut current);
                }
                current.push(member);
                prev_end = Some(range.end());
                prev_multiline = source.contains_line_break(range);
            }
            Slot::Bridge => {
                if let Some(end) = prev_end.as_mut() {
                    *end = range.end();
                }
            }
            Slot::Break => {
                flush_run(&mut groups, &mut current);
                prev_end = None;
                prev_multiline = false;
            }
        }
    }
    flush_run(&mut groups, &mut current);
    groups
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

/// Moves the in-progress run into `groups` when it holds at least one
/// member, leaving `current` empty for the next run.
fn flush_run<M>(groups: &mut Vec<Vec<M>>, current: &mut Vec<M>) {
    if !current.is_empty() {
        groups.push(std::mem::take(current));
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
    use crate::testing::parse;

    #[test]
    fn adjacent_member_groups_break_after_multiline_closes_run() {
        let source = parse("a = 1\nx = [\n    1,\n]\nc = 3\n");
        let mut index = 0usize;
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, true, |_| {
                let member = Slot::Member(index);
                index += 1;
                member
            });

        // The multi-line middle statement closes the run after it, so the
        // trailing member starts a fresh group.
        assert_eq!(groups, vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn adjacent_member_groups_break_ends_the_run() {
        let source = parse("a = 1\nb = 2\nc = 3\n");
        let mut index = 0usize;
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, false, |_| {
                let slot = if index == 1 {
                    Slot::Break
                } else {
                    Slot::Member(index)
                };
                index += 1;
                slot
            });

        // The Break at index 1 closes the run, leaving 0 and 2 in separate groups.
        assert_eq!(groups, vec![vec![0], vec![2]]);
    }

    #[test]
    fn adjacent_member_groups_bridge_does_not_trip_multiline_break() {
        let source = parse("a = 1\nb = [\n    1,\n]\nc = 3\n");
        let mut index = 0usize;
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, true, |_| {
                let slot = if index == 1 {
                    Slot::Bridge
                } else {
                    Slot::Member(index)
                };
                index += 1;
                slot
            });

        // The bridged middle statement spans lines, but a Bridge never sets
        // the multi-line flag, so 0 and 2 still span it as one block.
        assert_eq!(groups, vec![vec![0, 2]]);
    }

    #[test]
    fn adjacent_member_groups_bridge_spans_neighbors() {
        let source = parse("a = 1\nb = 2\nc = 3\n");
        let mut index = 0usize;
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, false, |_| {
                let slot = if index == 1 {
                    Slot::Bridge
                } else {
                    Slot::Member(index)
                };
                index += 1;
                slot
            });

        // The Bridge at index 1 passes the run through, so 0 and 2 align as one block.
        assert_eq!(groups, vec![vec![0, 2]]);
    }

    #[test]
    fn adjacent_member_groups_gathers_adjacent_members() {
        let source = parse("a = 1\nb = 2\nc = 3\n");
        let mut index = 0usize;
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, false, |_| {
                let member = Slot::Member(index);
                index += 1;
                member
            });

        assert_eq!(groups, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn adjacent_member_groups_splits_on_blank_line() {
        let source = parse("a = 1\n\nc = 3\n");
        let mut index = 0usize;
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, false, |_| {
                let member = Slot::Member(index);
                index += 1;
                member
            });

        // The blank line breaks adjacency, so the two members do not share a group.
        assert_eq!(groups, vec![vec![0], vec![1]]);
    }

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
}

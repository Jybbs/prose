//! Statement grouping for the alignment rules. Walks a body into
//! line-adjacent runs of qualified members, passing a skip-held row
//! through so its neighbors align as one block.

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
///
/// An item sharing its row with a sibling lands in a run of its own,
/// because a column belongs to a row that one member owns. Two items on
/// one row otherwise pair diagonally, the trailing item of one row
/// landing in a column with the leading item of the next. The lone run
/// still reaches the rules that read a single-member group, so a packed
/// row sheds stale padding while seating no column.
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
    let items: Vec<T> = items.into_iter().collect();
    let shared = shared_rows(source, &items);
    for (i, item) in items.into_iter().enumerate() {
        let range = item.range();
        let slot = classify(item);
        if shared[i] {
            flush_run(&mut groups, &mut current);
            prev_end = None;
            prev_multiline = false;
            if let Slot::Member(member) = slot {
                groups.push(vec![member]);
            }
            continue;
        }
        match slot {
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
/// member shares the active key, sits on the source line directly below
/// the prior statement, and the prior statement itself fits on one
/// source line. A key change at an otherwise-adjacent boundary closes
/// the active run and starts a fresh one without losing the boundary
/// statement. A single-line statement [held](is_held) for `rule` is
/// transparent, in that it joins no group and leaves neighbors on
/// either side to align as one block, whereas a held multi-line
/// statement stands as the prior statement and closes the run. A
/// trailing comment on a row sits inside that row, so it leaves the run
/// intact, while a standalone comment line or a blank line between rows
/// breaks it. Walks `body` exactly once.
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
    let mut active: Option<(K, TextRange)> = None;
    for stmt in body {
        let Some((key, member)) = qualify(stmt) else {
            flush_run(&mut groups, &mut current);
            active = None;
            continue;
        };
        if is_held(source, rule, stmt.start()) {
            if let Some((_, prev)) = active.as_mut() {
                *prev = stmt.range();
            }
            continue;
        }
        let extends = active.as_ref().is_some_and(|(active_key, prev)| {
            active_key == &key
                && !source.contains_line_break(prev)
                && source.consecutive_lines(prev.end(), stmt.start())
        });
        if !extends {
            flush_run(&mut groups, &mut current);
        }
        current.push(member);
        active = Some((key, stmt.range()));
    }
    flush_run(&mut groups, &mut current);
    groups
}

/// Walks `body`, qualifying each statement through `qualify` and
/// grouping the qualified members into runs where every consecutive
/// pair sits on adjacent source lines. A multi-line prior statement,
/// a non-qualifying statement, an own-line comment between two rows,
/// or a blank line breaks the current run. A single-line statement held
/// for `rule` is transparent per [`keyed_line_adjacent_groups`]. Empty
/// groups (statements that fail qualification with no qualified
/// neighbors) are skipped. Thin wrapper over
/// [`keyed_line_adjacent_groups`] for rules whose qualifier produces
/// only one form, so every member shares an implicit `()` key.
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

/// One flag per item, set where the item shares a source row with the
/// sibling on either side of it.
fn shared_rows<T: Ranged>(source: &Source, items: &[T]) -> Vec<bool> {
    let touching: Vec<bool> = items
        .windows(2)
        .map(|pair| source.same_line(pair[0].end(), pair[1].start()))
        .collect();
    (0..items.len())
        .map(|i| {
            i.checked_sub(1).is_some_and(|p| touching[p]) || touching.get(i).is_some_and(|&t| t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// A classifier handing `slot_of` each item's index in walk order.
    fn indexed<T>(mut slot_of: impl FnMut(usize) -> Slot<usize>) -> impl FnMut(T) -> Slot<usize> {
        let mut index = 0;
        move |_| {
            let slot = slot_of(index);
            index += 1;
            slot
        }
    }

    #[test]
    fn adjacent_member_groups_break_after_multiline_closes_run() {
        let source = parse("a = 1\nx = [\n    1,\n]\nc = 3\n");
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, true, indexed(Slot::Member));

        // The multi-line middle statement closes the run after it, so the
        // trailing member starts a fresh group.
        assert_eq!(groups, vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn adjacent_member_groups_break_ends_the_run() {
        let source = parse("a = 1\nb = 2\nc = 3\n");
        let groups: Vec<Vec<usize>> = adjacent_member_groups(
            &source,
            &source.ast().body,
            false,
            indexed(|index| {
                if index == 1 {
                    Slot::Break
                } else {
                    Slot::Member(index)
                }
            }),
        );

        // The Break at index 1 closes the run, leaving 0 and 2 in separate groups.
        assert_eq!(groups, vec![vec![0], vec![2]]);
    }

    #[test]
    fn adjacent_member_groups_bridge_does_not_trip_multiline_break() {
        let source = parse("a = 1\nb = [\n    1,\n]\nc = 3\n");
        let groups: Vec<Vec<usize>> = adjacent_member_groups(
            &source,
            &source.ast().body,
            true,
            indexed(|index| {
                if index == 1 {
                    Slot::Bridge
                } else {
                    Slot::Member(index)
                }
            }),
        );

        // The bridged middle statement spans lines, but a Bridge never sets
        // the multi-line flag, so 0 and 2 still span it as one block.
        assert_eq!(groups, vec![vec![0, 2]]);
    }

    #[test]
    fn adjacent_member_groups_bridge_spans_neighbors() {
        let source = parse("a = 1\nb = 2\nc = 3\n");
        let groups: Vec<Vec<usize>> = adjacent_member_groups(
            &source,
            &source.ast().body,
            false,
            indexed(|index| {
                if index == 1 {
                    Slot::Bridge
                } else {
                    Slot::Member(index)
                }
            }),
        );

        // The Bridge at index 1 passes the run through, so 0 and 2 align as one block.
        assert_eq!(groups, vec![vec![0, 2]]);
    }

    #[test]
    fn adjacent_member_groups_gathers_adjacent_members() {
        let source = parse("a = 1\nb = 2\nc = 3\n");
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, false, indexed(Slot::Member));

        assert_eq!(groups, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn adjacent_member_groups_splits_on_blank_line() {
        let source = parse("a = 1\n\nc = 3\n");
        let groups: Vec<Vec<usize>> =
            adjacent_member_groups(&source, &source.ast().body, false, indexed(Slot::Member));

        // The blank line breaks adjacency, so the two members do not share a group.
        assert_eq!(groups, vec![vec![0], vec![1]]);
    }

    #[rstest]
    #[case::lone_qualifier("x = 1\n", vec![1])]
    #[case::adjacent_same_key("x = 1\ny = 2\nz = 3\n", vec![3])]
    #[case::trailing_active_run("x = 1\ny = 2\n", vec![2])]
    #[case::empty_body("", vec![])]
    #[case::blank_line("x = 1\n\ny = 2\n", vec![1, 1])]
    #[case::comment_in_gap("x = 1\n# comment\ny = 2\n", vec![1, 1])]
    #[case::multiline_prior_stmt("x = {\n    'a': 1,\n}\ny = 2\n", vec![1, 1])]
    #[case::non_qualifier("x = 1\npass\ny = 2\n", vec![1, 1])]
    #[case::trailing_comment("x = 1  # note\ny = 2\nz = 3\n", vec![3])]
    #[case::held_row_bridges("x = 1\ny = 2  # prose: skip[align-equals]\nz = 3\n", vec![2])]
    #[case::held_row_with_extra_comment(
        "x = 1\ny = 2  # note  # prose: skip[align-equals]\nz = 3\n",
        vec![2]
    )]
    #[case::blank_line_after_held(
        "x = 1\ny = 2  # prose: skip[align-equals]\n\nz = 3\n",
        vec![1, 1]
    )]
    #[case::standalone_comment_after_held(
        "x = 1\ny = 2  # prose: skip[align-equals]\n# note\nz = 3\n",
        vec![1, 1]
    )]
    #[case::held_multiline_stmt(
        "x = 1\ny = {\n    'a': 1,\n}  # prose: skip[align-equals]\nz = 3\n",
        vec![1, 1]
    )]
    fn keyed_line_adjacent_groups_partitions_by_adjacency(
        #[case] src: &str,
        #[case] expected: Vec<usize>,
    ) {
        let source = parse(src);
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), expected);
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
}

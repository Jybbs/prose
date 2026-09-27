//! Forecasts where the layout rules break a statement run's values open
//! to join its column, cutting the run after each row they break, so the
//! columns the aligner reports for the run match the layout those rules
//! leave.

use super::{
    Member, Settings, Widenings,
    emit::{
        Extent, columns, emitted_base_width, emitted_extents, group_columns, group_max_width,
        max_op_width, padded_members, padding_width, reading_order_groups, tally,
    },
    holds::is_alignment_candidate,
};
use crate::source::Source;

/// A statement whose value a layout rule can break open, as the source
/// writes its row: `code` the width the row's code reaches on one line,
/// a trailing comment left out, and `opener` the width the row reaches
/// through the bracket the rule breaks the value open at.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Breakable {
    pub(crate) code: usize,
    pub(crate) opener: usize,
}

/// A statement row as the rules seated ahead of the alignment leave it:
/// `shift` how far an earlier column moves everything past the row's
/// name, `breaks` the widths the row reaches where a layout rule can
/// break its value open, and `holds` set where breaking it open would
/// split that earlier column's run, so the row breaks only where its
/// code crosses the cap at the buffer alone.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Statement {
    pub(crate) breaks: Option<Breakable>,
    pub(crate) holds: bool,
    pub(crate) shift: isize,
}

/// A statement run as [`breaking_columns`] reads it, each breakable row
/// measured the way the aligner emits its line.
struct Cuts<'a> {
    extents: &'a [Extent],
    line_length: usize,
    members: &'a [Member],
    rows: &'a [Option<Breakable>],
    settings: Settings,
    statements: &'a [Statement],
}

impl Cuts<'_> {
    /// Computes each member's column once the run cuts after each row the
    /// layout rules break open, taking the cuts that leave the fewest groups,
    /// then the fewest rows unpadded, then the fewest broken rows. Only a row
    /// crossing the cap inside the run's width spread can break, and a held
    /// row only where it crosses the cap at the buffer alone.
    fn columns(&self) -> Vec<usize> {
        let count = self.members.len();
        let widest = group_max_width(self.members);
        let widest_op = max_op_width(self.members);
        let breakable: Vec<usize> = (0..count)
            .filter(|&index| {
                self.rows[index].is_some_and(|row| {
                    let padding = if self.statements[index].holds {
                        self.settings.buffer
                    } else {
                        padding_width(self.members[index], widest, widest_op, self.settings.buffer)
                    };
                    self.overflows(row, padding)
                })
            })
            .collect();
        if breakable.is_empty() {
            return group_columns(self.members, self.extents, self.settings);
        }
        let starts = std::iter::once(0).chain(breakable.iter().map(|&end| end + 1));
        let mut best: Vec<Option<(Score, Option<usize>)>> = vec![None; count + 1];
        best[count] = Some(((0, 0, 0), None));
        for start in starts.filter(|&start| start < count).rev() {
            let whole = self
                .score(start, count - 1, false)
                .map(|(groups, unpadded)| ((groups, unpadded, 0), None));
            let cut = breakable
                .iter()
                .filter(|&&end| end >= start)
                .filter_map(|&end| {
                    let (groups, unpadded) = self.score(start, end, true)?;
                    let ((rest_groups, rest_unpadded, rest_breaks), _) = best[end + 1]?;
                    Some((
                        (
                            groups + rest_groups,
                            unpadded + rest_unpadded,
                            rest_breaks + 1,
                        ),
                        Some(end),
                    ))
                });
            best[start] = whole.into_iter().chain(cut).min_by_key(|&(score, _)| score);
        }
        let mut columns = Vec::with_capacity(count);
        let mut start = 0;
        while start < count {
            let (_, cut) = best[start].expect(
                "invariant: each start holds a score, every row overflowing its group's padding being breakable",
            );
            let (end, broken) = cut.map_or((count - 1, false), |end| (end, true));
            columns.extend(group_columns(
                &self.members[start..=end],
                &self.segment(start, end, broken),
                self.settings,
            ));
            start = end + 1;
        }
        columns
    }

    /// Reports whether `row`'s code crosses the cap once `padding` stands
    /// ahead of its operator.
    fn overflows(&self, row: Breakable, padding: usize) -> bool {
        row.code + padding > self.line_length
    }

    /// Counts the groups and the rows standing unpadded, per [`tally`], of
    /// the members from `start` through `end` read as one run, the row at
    /// `end` broken open where `broken` is set. Returns `None` where the
    /// layout rules would break a row other than that one, or leave that one
    /// on its row, at the padding its group gives it.
    fn score(&self, start: usize, end: usize, broken: bool) -> Option<(usize, usize)> {
        let extents = self.segment(start, end, broken);
        let groups = reading_order_groups(&self.members[start..=end], &extents, self.settings);
        groups
            .iter()
            .flat_map(|&(group, widest)| padded_members(group, widest, self.settings))
            .zip(start..)
            .all(|((_, padding), index)| {
                self.rows[index]
                    .is_none_or(|row| self.overflows(row, padding) == (broken && index == end))
            })
            .then(|| tally(&groups))
    }

    /// Collects the extents of the members from `start` through `end`, the
    /// row at `end` read at its opening row alone where `broken` is set.
    fn segment(&self, start: usize, end: usize, broken: bool) -> Vec<Extent> {
        let mut extents = self.extents[start..=end].to_vec();
        if let Some(row) = self.rows[end].filter(|_| broken) {
            extents[end - start] = Extent {
                expanded: None,
                inline: row.opener,
            };
        }
        extents
    }
}

/// The groups, the rows standing unpadded, and the broken rows one way of
/// cutting a run leaves, compared in that order.
type Score = (usize, usize, usize);

/// Computes [`operator_columns`](super::operator_columns) for a run of
/// statements, each of `statements` in step with `members` shifting its
/// row and naming whether a layout rule can break its value open. A value
/// breaks open, ending its run, where its row crosses the cap at its
/// group's column and where [`Cuts`] finds that leaves fewer groups, or as
/// many with fewer rows unpadded, than keeping it on its row.
pub(crate) fn breaking_columns(
    source: &Source,
    members: &[Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
    statements: &[Statement],
) -> Vec<usize> {
    let members: Vec<Member> = members
        .iter()
        .zip(statements)
        .map(|(&member, statement)| {
            member.with_settled_width(member.settled_width.saturating_add_signed(statement.shift))
        })
        .collect();
    let candidate = is_alignment_candidate(&members);
    let Some(cap) = settings.cap.filter(|_| candidate) else {
        return columns(source, &members, settings, widenings, joined, candidate);
    };
    let emitted = |member: Member, width: usize, statement: &Statement| {
        emitted_base_width(source, member, cap, Some(width))
            .saturating_add_signed(widenings.delta(member) + statement.shift)
    };
    let extents: Vec<Extent> = emitted_extents(source, &members, settings, widenings, joined)
        .into_iter()
        .zip(statements)
        .map(|(extent, statement)| Extent {
            inline: extent.inline.saturating_add_signed(statement.shift),
            ..extent
        })
        .collect();
    let rows: Vec<Option<Breakable>> = members
        .iter()
        .zip(statements)
        .map(|(&member, statement)| {
            statement.breaks.map(|row| Breakable {
                code: emitted(member, row.code, statement),
                opener: emitted(member, row.opener, statement),
            })
        })
        .collect();
    Cuts {
        extents: &extents,
        line_length: cap.line_length,
        members: &members,
        rows: &rows,
        settings,
        statements,
    }
    .columns()
}

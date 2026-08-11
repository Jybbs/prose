//! Predicts the widening the rule's own later groups seat on each
//! line, so a column decided early measures a line at the width the
//! pass leaves it rather than the width the source carries.

use ruff_text_size::{TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::{Member, Settings};
use crate::source::Source;

/// The widening each collected member seats on its own line, the gap
/// ahead of the token brought to the settings' buffer and the
/// post-operator gap brought to one space. A line-cap check adds the
/// entries of the other members sharing a member's line. No entry goes
/// below zero.
#[derive(Default)]
pub(crate) struct Widenings(Vec<(TextSize, TextRange, isize)>);

impl Widenings {
    /// Builds the widening entries for `members` under `settings`,
    /// dropping zero entries.
    pub(crate) fn of(
        source: &Source,
        settings: Settings,
        members: impl Iterator<Item = Member>,
    ) -> Self {
        let mut entries: Vec<(TextSize, TextRange, isize)> = members
            .filter_map(|m| {
                let gap_part =
                    settings.buffer.cast_signed() - source.slice(m.gap).width().cast_signed();
                let value_part = m
                    .rewritten_value_gap(source)
                    .map_or(0, |g| 1 - source.slice(g).width().cast_signed());
                let delta = (gap_part + value_part).max(0);
                (delta != 0).then_some((m.line_start, m.gap, delta))
            })
            .collect();
        entries.sort_unstable_by_key(|&(line, gap, _)| (line, gap.start()));
        Self(entries)
    }

    /// The widening the other collected members seat on `member`'s
    /// line, zero where none share it.
    pub(crate) fn delta(&self, member: Member) -> isize {
        let from = self
            .0
            .partition_point(|&(line, ..)| line < member.line_start);
        self.0[from..]
            .iter()
            .take_while(|&&(line, ..)| line == member.line_start)
            .filter(|&&(_, gap, _)| gap != member.gap)
            .map(|&(.., delta)| delta)
            .sum()
    }
}

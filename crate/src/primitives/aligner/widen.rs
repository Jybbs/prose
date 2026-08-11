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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::AlignmentConfig,
        testing::{align_member, parse, range},
    };

    fn settings() -> Settings {
        Settings::from(&AlignmentConfig::default())
    }

    #[test]
    fn delta_answers_zero_for_a_member_alone_on_its_line() {
        let source = parse("a=1\nbc=2\n");
        let members = [
            align_member(range(1, 1), 0, 1),
            align_member(range(6, 6), 4, 2),
        ];
        let widenings = Widenings::of(&source, settings(), members.iter().copied());
        assert_eq!(widenings.delta(members[0]), 0);
    }

    #[test]
    fn delta_sums_only_the_other_members_sharing_the_line() {
        let source = parse("f(a=1, b=2)\n");
        let members = [
            align_member(range(3, 3), 0, 1),
            align_member(range(8, 8), 0, 1),
        ];
        let widenings = Widenings::of(&source, settings(), members.iter().copied());
        assert_eq!(widenings.delta(members[0]), 1);
    }

    #[test]
    fn of_drops_a_member_already_at_the_buffer() {
        let source = parse("a =1\n");
        let members = [align_member(range(1, 2), 0, 1)];
        let widenings = Widenings::of(&source, settings(), members.iter().copied());
        assert!(widenings.0.is_empty());
    }
}

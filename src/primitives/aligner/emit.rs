//! Padding-width math and edit emission for the alignment rules.
//! Dispatches a group through its `max-shift` policy and rewrites each
//! member's gap to the computed column.

use std::cmp::Reverse;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use super::{Member, Settings};
use crate::{config::MaxAlignShiftPolicy, source::Source};

/// Aligns the members of a group, dispatching through `settings.policy`
/// when the widest padding exceeds `settings.max_shift`. A singleton
/// group collapses its gap to one space, or to zero when
/// `settings.strip_singleton_subgroup` is set, matching the
/// singleton-from-split treatment in `emit_split`.
pub(super) fn emit_group(
    source: &Source,
    members: &[Member],
    settings: Settings,
    edits: &mut Vec<Edit>,
) {
    let Some((min_w, max_w)) = members.iter().map(|m| m.width).minmax().into_option() else {
        return;
    };
    if max_w - min_w <= settings.max_shift {
        let suffix = settings.suffix_len(members.len());
        emit_with_paddings(source, members, max_w, max_op_width(members), suffix, edits);
        return;
    }
    match settings.policy {
        MaxAlignShiftPolicy::Drop => emit_drop(source, members, settings, edits),
        MaxAlignShiftPolicy::Split => emit_split(source, members, settings, edits),
    }
}

/// Returns the edit needed to make `range` carry exactly `n` ASCII
/// spaces, or `None` if it already does. Emits `Edit::range_deletion`
/// when `n` is zero.
pub(crate) fn space_padding_edit(source: &Source, range: TextRange, n: usize) -> Option<Edit> {
    let text = source.slice(range);
    if text.len() == n && text.bytes().all(|b| b == b' ') {
        return None;
    }
    if n == 0 {
        return Some(Edit::range_deletion(range));
    }
    Some(Edit::range_replacement(" ".repeat(n), range))
}

/// Returns the length of the prefix of `members` whose widths sit
/// within `max_shift` of `anchor_width`. The slice must be sorted so
/// distance from the anchor grows monotonically.
fn band_len(members: &[Member], anchor_width: usize, max_shift: usize) -> usize {
    members.partition_point(|m| m.width.abs_diff(anchor_width) <= max_shift)
}

/// Sorts by width, keeps only the members whose width sits within
/// `max_shift` of the minimum, and aligns that subset. Excluded
/// members retain their original spacing. A kept set of fewer than
/// two members emits nothing.
fn emit_drop(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
    let mut sorted = members.to_vec();
    sorted.sort_unstable_by_key(|m| m.width);
    let min = sorted
        .first()
        .expect("emit_drop invariant: members is non-empty")
        .width;
    let kept_end = band_len(&sorted, min, settings.max_shift);
    let kept = &sorted[..kept_end];
    if kept.len() < 2 {
        return;
    }
    let max_w = kept.last().expect("kept non-empty").width;
    emit_with_paddings(source, kept, max_w, max_op_width(kept), 1, edits);
}

/// Partitions into width bands, seeding each at the widest unassigned
/// member and claiming every member whose width sits within
/// `settings.max_shift` of the seed, so the dominant column is sized
/// by the members that need it and a member lands alone only as a
/// width outlier among the members not yet claimed by a wider band.
/// Each band aligns at its seed's width. A singleton collapses its
/// gap to one space, or to zero when
/// `settings.strip_singleton_subgroup` is set.
fn emit_split(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
    let mut sorted = members.to_vec();
    sorted.sort_unstable_by_key(|m| Reverse(m.width));
    let mut rest = sorted.as_slice();
    while let Some(seed) = rest.first() {
        let end = band_len(rest, seed.width, settings.max_shift);
        let (band, tail) = rest.split_at(end);
        let suffix = settings.suffix_len(band.len());
        emit_with_paddings(source, band, seed.width, max_op_width(band), suffix, edits);
        rest = tail;
    }
}

/// Rewrites each member's gap to
/// `suffix_len + (max_w - m.width) + (max_op_w - m.op_width)` spaces.
/// The operator-width term right-aligns variable-width operators so
/// each operator's last character lands in the shared column.
/// Members whose gap already carries that width of ASCII spaces emit
/// nothing.
fn emit_with_paddings(
    source: &Source,
    members: &[Member],
    max_w: usize,
    max_op_w: usize,
    suffix_len: usize,
    edits: &mut Vec<Edit>,
) {
    edits.extend(members.iter().filter_map(|m| {
        space_padding_edit(
            source,
            m.gap,
            suffix_len + (max_w - m.width) + (max_op_w - m.op_width),
        )
    }));
}

/// Returns the widest `op_width` in `members`, or `0` when the slice
/// is empty.
fn max_op_width(members: &[Member]) -> usize {
    members.iter().map(|m| m.op_width).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use ruff_text_size::{Ranged, TextSize};

    use super::*;
    use crate::testing::parse;

    /// Builds the expected summary tuple for an `Edit::range_deletion`
    /// over a member's gap.
    fn delete(member: &Member) -> (u32, u32, String) {
        (
            member.gap.start().to_u32(),
            member.gap.end().to_u32(),
            String::new(),
        )
    }

    /// Builds the expected `(start, end, content)` tuple for an edit
    /// that rewrites a member's gap to `n` spaces.
    fn fill(member: &Member, n: usize) -> (u32, u32, String) {
        (
            member.gap.start().to_u32(),
            member.gap.end().to_u32(),
            " ".repeat(n),
        )
    }

    /// Builds a multi-line Python source where each row is
    /// `x...x{spaces}= 0\n`, returns the source plus one `Member` per
    /// row pointing at that row's pre-`=` whitespace. `gap_chars` seeds
    /// the existing pre-`=` whitespace.
    fn rows(specs: &[(usize, usize)]) -> (Source, Vec<Member>) {
        let offset = |t: &str| TextSize::try_from(t.len()).expect("test source fits in u32");
        let mut text = String::new();
        let mut members = Vec::new();
        for &(width, gap_chars) in specs {
            let line_start = offset(&text);
            text.push_str(&"x".repeat(width));
            let gap_start = offset(&text);
            text.push_str(&" ".repeat(gap_chars));
            let gap_end = offset(&text);
            text.push_str("= 0\n");
            members.push(Member {
                gap: TextRange::new(gap_start, gap_end),
                line_start,
                op_width: 0,
                width,
            });
        }
        (parse(&text), members)
    }

    /// Builds a `Settings` carrying the test's cap and policy with
    /// `strip_singleton_subgroup` defaulted off.
    fn settings(max_shift: usize, policy: MaxAlignShiftPolicy) -> Settings {
        Settings::aligned(
            NonZeroUsize::new(max_shift).expect("test cap is non-zero"),
            policy,
        )
    }

    fn sorted_summaries(edits: &[Edit]) -> Vec<(u32, u32, String)> {
        let mut out: Vec<_> = edits.iter().map(summary).collect();
        out.sort();
        out
    }

    /// Pulls a sortable `(start, end, content)` tuple out of an `Edit`.
    fn summary(edit: &Edit) -> (u32, u32, String) {
        (
            edit.start().to_u32(),
            edit.end().to_u32(),
            edit.content().unwrap_or_default().to_owned(),
        )
    }

    #[test]
    fn emit_group_aligns_to_shared_column_when_spread_fits_under_cap() {
        let (source, members) = rows(&[(1, 1), (2, 1), (3, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(10, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        // max_w=3, paddings 2/1/0, suffix=1 → targets 3/2/1 spaces.
        // member[2] already has 1 space, so it is skipped.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 3), fill(&members[1], 2)],
        );
    }

    #[test]
    fn emit_group_collapses_single_member_gap_to_suffix_len() {
        let (source, members) = rows(&[(3, 5)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        // single member fits any cap. max_w=3, padding=0, suffix=1 →
        // target 1 space, currently 5.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 1)]);
    }

    #[test]
    fn emit_group_drop_emits_nothing_when_kept_set_under_two() {
        let (source, members) = rows(&[(1, 1), (15, 1), (30, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(4, MaxAlignShiftPolicy::Drop),
            &mut edits,
        );

        assert!(
            edits.is_empty(),
            "drop policy must emit nothing when fewer than two members fit the cap",
        );
    }

    #[test]
    fn emit_group_drop_excludes_outliers_and_aligns_remainder() {
        let (source, members) = rows(&[(1, 1), (2, 1), (15, 1), (3, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Drop),
            &mut edits,
        );

        // widths sort to 1/2/3/15. cap=8, so kept = widths 1/2/3 (the
        // width-15 outlier drops). max_w of kept = 3, paddings 2/1/0,
        // targets 3/2/1 spaces. member[3] (width 3) already has 1 space.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 3), fill(&members[1], 2)],
        );
    }

    #[test]
    fn emit_group_drop_keeps_member_exactly_at_cap_boundary() {
        // Width 9 sits exactly max_shift=8 from the width-1 minimum, so
        // the inclusive band keeps it and the pair aligns at 9.
        let (source, members) = rows(&[(1, 1), (9, 1), (30, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Drop),
            &mut edits,
        );

        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 9)]);
    }

    #[test]
    fn emit_group_handles_empty_member_slice() {
        let source = parse("x = 0\n");
        let mut edits = Vec::new();

        emit_group(
            &source,
            &[],
            settings(8, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        assert!(edits.is_empty());
    }

    #[test]
    fn emit_group_split_aligns_band_across_interleaved_outlier() {
        // Widths span 13 → 4, a 9-wide spread that exceeds max_shift=8.
        // The width-13 seed claims 11 and 7, so the band aligns at 13
        // around the interleaved width-4 outlier, which strips to a
        // zero-width gap.
        let (source, members) = rows(&[(13, 1), (4, 1), (11, 1), (7, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // member[0] is the seed, already at gap=1.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                delete(&members[1]),
                fill(&members[2], 3),
                fill(&members[3], 7),
            ],
        );
    }

    #[test]
    fn emit_group_split_bands_equal_widths_and_strips_outlier() {
        // The two width-13 members band together regardless of the
        // width-4 row between them, leaving that row the lone stripped
        // singleton.
        let (source, members) = rows(&[(13, 1), (4, 1), (13, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // Both width-13 gaps already sit at one space.
        assert_eq!(sorted_summaries(&edits), vec![delete(&members[1])]);
    }

    #[test]
    fn emit_group_split_claims_mid_width_into_widest_band() {
        // The width-20 seed claims 12 (spread 8 fits the cap), so 4
        // lands alone even though 12 sits within the cap of its own
        // width: banding is greedy from the widest unassigned member.
        let (source, members) = rows(&[(20, 1), (12, 1), (4, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[1], 9), delete(&members[2])],
        );
    }

    #[test]
    fn emit_group_split_leaves_over_cap_pair_natural() {
        let (source, members) = rows(&[(20, 1), (4, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        // Each member is its own band. Without strip, both singleton
        // targets are the one-space gap each row already carries.
        assert!(edits.is_empty());
    }

    #[test]
    fn emit_group_split_partitions_by_width_not_source_order() {
        let (source, members) = rows(&[(1, 1), (2, 1), (15, 1), (3, 1), (4, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        // bands: [15] singleton at its natural one-space gap, then
        // [4, 3, 2, 1] aligns at 4. Targets are 4/3/1/2/1 spaces, and
        // members 2 and 4 already carry theirs.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[0], 4),
                fill(&members[1], 3),
                fill(&members[3], 2),
            ],
        );
    }

    #[test]
    fn emit_group_split_partitions_into_three_bands() {
        // Widths 30/25 band at the cap, 15/10 band next, and 1 lands
        // alone: the loop must keep seeding past the second band.
        let (source, members) = rows(&[(30, 1), (25, 1), (15, 1), (10, 1), (1, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // Band [30, 25] aligns at 30, band [15, 10] aligns at 15, and
        // [1] is the stripped singleton. Members 0 and 2 are seeds
        // already at gap=1.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[1], 6),
                fill(&members[3], 6),
                delete(&members[4]),
            ],
        );
    }

    #[test]
    fn emit_group_split_right_aligns_operators_within_band() {
        let (source, members) = rows(&[(12, 1), (11, 1), (1, 1)]);
        let members = [
            members[0].with_op_width(2),
            members[1].with_op_width(1),
            members[2].with_op_width(1),
        ];
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        // Band [12, 11] right-aligns on its own widest operator, so
        // member[1] targets 1+1+1=3 spaces while member[0] keeps its
        // one-space gap and the [1] singleton takes no operator padding
        // from the wide band.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[1], 3)]);
    }

    #[test]
    fn emit_group_split_strips_singleton_subgroup_when_flag_is_set() {
        let (source, members) = rows(&[(10, 1), (11, 1), (1, 1), (12, 1), (13, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // The width-13 seed claims 12, 11, and 10 into one band aligned
        // at 13, while [1] is the stripped singleton.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[0], 4),
                fill(&members[1], 3),
                delete(&members[2]),
                fill(&members[3], 2),
            ],
        );
    }

    #[test]
    fn emit_group_strips_lone_member_gap_when_flag_is_set() {
        let (source, members) = rows(&[(3, 5)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // A lone member is its own group, so strip collapses the
        // five-space gap to zero rather than the one-space suffix.
        assert_eq!(sorted_summaries(&edits), vec![delete(&members[0])]);
    }

    #[test]
    fn emit_with_paddings_emits_deletion_when_target_len_is_zero() {
        let (source, members) = rows(&[(3, 4)]);
        let mut edits = Vec::new();

        // max_w == m.width → padding = 0, suffix_len = 0 → target_len = 0
        // → must emit deletion (range_replacement rejects empty content).
        emit_with_paddings(&source, &members, members[0].width, 0, 0, &mut edits);

        assert_eq!(edits.len(), 1);
        assert_eq!(
            summary(&edits[0]),
            (
                members[0].gap.start().to_u32(),
                members[0].gap.end().to_u32(),
                String::new()
            )
        );
    }

    #[test]
    fn emit_with_paddings_skips_already_correct_gap() {
        let (source, members) = rows(&[(3, 1)]);
        let mut edits = Vec::new();

        // max_w == m.width → padding = 0, suffix_len = 1 → target_len = 1.
        // The fabricated gap is one space already, so emit nothing.
        emit_with_paddings(&source, &members, members[0].width, 0, 1, &mut edits);

        assert!(
            edits.is_empty(),
            "gap that already matches the target width must not emit",
        );
    }

    #[test]
    fn space_padding_edit_inserts_when_range_empty_and_n_positive() {
        let source = parse("xy\n");
        let range = TextRange::new(TextSize::new(1), TextSize::new(1));
        let edit = space_padding_edit(&source, range, 2).expect("0-vs-2 spaces emits");
        assert_eq!(summary(&edit), (1, 1, "  ".to_owned()));
    }

    #[test]
    fn space_padding_edit_replaces_when_text_has_non_space_chars() {
        let source = parse("a:b\n");
        let range = TextRange::new(TextSize::new(1), TextSize::new(2));
        let edit = space_padding_edit(&source, range, 1).expect("non-space content emits");
        assert_eq!(summary(&edit), (1, 2, " ".to_owned()));
    }

    #[test]
    fn space_padding_edit_returns_none_for_empty_range_at_zero() {
        let source = parse("xy\n");
        let range = TextRange::new(TextSize::new(1), TextSize::new(1));
        assert!(space_padding_edit(&source, range, 0).is_none());
    }
}

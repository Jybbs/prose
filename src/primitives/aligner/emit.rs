//! Padding-width math and edit emission for the alignment rules.
//! Dispatches a group through its `max-shift` policy and rewrites each
//! member's gap to the computed column.

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
    let Some(first) = members.first() else {
        return;
    };
    let (min_w, max_w) = members
        .iter()
        .fold((first.width, first.width), |(mn, mx), m| {
            (mn.min(m.width), mx.max(m.width))
        });
    let max_op_w = max_op_width(members);
    if max_w - min_w <= settings.max_shift {
        let suffix = if members.len() == 1 && settings.strip_singleton_subgroup {
            0
        } else {
            1
        };
        emit_with_paddings(source, members, max_w, max_op_w, suffix, edits);
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
    let kept_end = sorted.partition_point(|m| m.width <= min + settings.max_shift);
    let kept = &sorted[..kept_end];
    if kept.len() < 2 {
        return;
    }
    let max_w = kept.last().expect("kept non-empty").width;
    emit_with_paddings(source, kept, max_w, max_op_width(kept), 1, edits);
}

/// Partitions greedily into sub-groups capped at `settings.max_shift`
/// spread, each aligning at its own widest member. A singleton
/// collapses its gap to one space, or to zero when
/// `settings.strip_singleton_subgroup` is set.
fn emit_split(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
    let subs = partition_by_spread(members, settings.max_shift);
    for &(start, end, sub_max_w) in &subs {
        let sub = &members[start..end];
        let (max_w, suffix) = if sub.len() == 1 && settings.strip_singleton_subgroup {
            (sub[0].width, 0)
        } else {
            (sub_max_w, 1)
        };
        emit_with_paddings(source, sub, max_w, max_op_width(sub), suffix, edits);
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

/// Returns the half-open `(start, end, max_width)` sub-group ranges
/// into `members` produced by greedily extending each sub-group while
/// the running `max_width - min_width` stays at or below `max_shift`.
fn partition_by_spread(members: &[Member], max_shift: usize) -> Vec<(usize, usize, usize)> {
    let mut subs = Vec::new();
    let mut cursor = 0;
    while cursor < members.len() {
        let mut min_w = members[cursor].width;
        let mut max_w = min_w;
        let mut end = cursor + 1;
        while end < members.len() {
            let w = members[end].width;
            let new_min = min_w.min(w);
            let new_max = max_w.max(w);
            if new_max - new_min > max_shift {
                break;
            }
            min_w = new_min;
            max_w = new_max;
            end += 1;
        }
        subs.push((cursor, end, max_w));
        cursor = end;
    }
    subs
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use ruff_text_size::{Ranged, TextSize};

    use super::*;
    use crate::testing::{parse, range};

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
        let mut text = String::new();
        let mut members = Vec::new();
        for &(width, gap_chars) in specs {
            let line_start = u32::try_from(text.len()).expect("test source fits in u32");
            for _ in 0..width {
                text.push('x');
            }
            let gap_start = u32::try_from(text.len()).expect("test source fits in u32");
            for _ in 0..gap_chars {
                text.push(' ');
            }
            let gap_end = u32::try_from(text.len()).expect("test source fits in u32");
            text.push_str("= 0\n");
            members.push(Member {
                gap: range(gap_start, gap_end),
                line_start: TextSize::new(line_start),
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
    fn emit_group_split_distances_widest_singleton_when_strip_is_set() {
        // Widths span 13 → 4, a 9-wide spread that exceeds max_shift=8.
        // The greedy partition isolates the leading width-13 member as
        // a singleton. With strip on, that singleton's gap collapses to
        // zero so its `:` sits distanced, while the [4, 11, 7] remainder
        // aligns within its own max_w of 11.
        let (source, members) = rows(&[(13, 1), (4, 1), (11, 1), (7, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // member[2] is the remainder's widest, already at gap=1.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                delete(&members[0]),
                fill(&members[1], 8),
                fill(&members[3], 5),
            ],
        );
    }

    #[test]
    fn emit_group_split_partitions_into_contiguous_subgroups() {
        let (source, members) = rows(&[(1, 1), (2, 1), (15, 1), (3, 1), (4, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Split),
            &mut edits,
        );

        // sub-groups: [1,2] aligns at max=2, [15] singleton, [3,4]
        // aligns at max=4. After alignment, targets are 2/1/1/2/1
        // spaces. members 1, 2, 4 already have 1 space.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 2), fill(&members[3], 2)],
        );
    }

    #[test]
    fn emit_group_split_strips_every_isolated_singleton() {
        // widths 13, 4, 13 yield spread = 9, exceeding max_shift=8, and
        // the greedy partition isolates each as its own singleton. With
        // strip on, every singleton's gap collapses to zero.
        let (source, members) = rows(&[(13, 1), (4, 1), (13, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        assert_eq!(
            sorted_summaries(&edits),
            vec![
                delete(&members[0]),
                delete(&members[1]),
                delete(&members[2]),
            ],
        );
    }

    #[test]
    fn emit_group_split_strips_singleton_subgroup_when_flag_is_set() {
        let (source, members) = rows(&[(10, 1), (11, 1), (1, 1), (12, 1), (13, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // [1] is a narrow singleton, so strip collapses its gap to 0
        // while [10, 11] and [12, 13] align within their own max_w.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[0], 2),
                delete(&members[2]),
                fill(&members[3], 2),
            ],
        );
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
        let range = range(1, 1);
        let edit = space_padding_edit(&source, range, 2).expect("0-vs-2 spaces emits");
        assert_eq!(summary(&edit), (1, 1, "  ".to_owned()));
    }

    #[test]
    fn space_padding_edit_replaces_when_text_has_non_space_chars() {
        let source = parse("a:b\n");
        let range = range(1, 2);
        let edit = space_padding_edit(&source, range, 1).expect("non-space content emits");
        assert_eq!(summary(&edit), (1, 2, " ".to_owned()));
    }

    #[test]
    fn space_padding_edit_returns_none_for_empty_range_at_zero() {
        let source = parse("xy\n");
        let range = range(1, 1);
        assert!(space_padding_edit(&source, range, 0).is_none());
    }
}

//! Padding-width math and edit emission for the alignment rules.
//! Splits each source-ordered run into the contiguous groups the
//! `max-shift` cap allows and rewrites each member's gap to its
//! group's column.

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use super::{Member, Settings};
use crate::{config::MaxShift, source::Source};

/// Aligns `members` by splitting the source-ordered run into the
/// contiguous groups `reading_order_groups` yields and emitting each at
/// its widest member. A singleton group collapses its gap to one space,
/// or to zero when `settings.strip_singleton` is set.
pub(super) fn emit_group(
    source: &Source,
    members: &[Member],
    settings: Settings,
    edits: &mut Vec<Edit>,
) {
    for (group, max_w) in reading_order_groups(members, settings.max_shift) {
        let suffix = settings.suffix_len(group.len());
        emit_with_paddings(source, group, max_w, max_op_width(group), suffix, edits);
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

/// Splits the source-ordered `members` into the contiguous groups the
/// aligner emits independently, each paired with its widest member's
/// width. `Unlimited` gathers the whole run into one group, `NoShift`
/// leaves every row its own singleton, and `Cap(n)` grows a group while
/// its width spread stays within `n`, cutting a fresh group at the first
/// row that would push the spread past it. Each group is a sub-slice, so
/// a column never jumps a row it skipped.
fn reading_order_groups(members: &[Member], max_shift: MaxShift) -> Vec<(&[Member], usize)> {
    let cap = match max_shift {
        MaxShift::NoShift => {
            return members
                .iter()
                .map(|m| (std::slice::from_ref(m), m.width))
                .collect();
        }
        MaxShift::Unlimited => usize::MAX,
        MaxShift::Cap(n) => n.get(),
    };
    let mut groups = Vec::new();
    let mut start = 0;
    let (mut min_w, mut max_w) = (usize::MAX, usize::MIN);
    for (i, member) in members.iter().enumerate() {
        let lo = min_w.min(member.width);
        let hi = max_w.max(member.width);
        if hi - lo > cap {
            groups.push((&members[start..i], max_w));
            (start, min_w, max_w) = (i, member.width, member.width);
        } else {
            (min_w, max_w) = (lo, hi);
        }
    }
    if start < members.len() {
        groups.push((&members[start..], max_w));
    }
    groups
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use ruff_text_size::{Ranged, TextSize};

    use super::*;
    use crate::testing::{parse, range};

    /// Builds a `MaxShift::Cap` from a non-zero literal.
    fn cap(n: usize) -> MaxShift {
        MaxShift::Cap(NonZeroUsize::new(n).expect("test cap is non-zero"))
    }

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

        emit_group(&source, &members, Settings::aligned(cap(10)), &mut edits);

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

        emit_group(&source, &members, Settings::aligned(cap(8)), &mut edits);

        // single member fits any cap. max_w=3, padding=0, suffix=1 →
        // target 1 space, currently 5.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 1)]);
    }

    #[test]
    fn emit_group_handles_empty_member_slice() {
        let source = parse("x = 0\n");
        let mut edits = Vec::new();

        emit_group(&source, &[], Settings::aligned(cap(8)), &mut edits);

        assert!(edits.is_empty());
    }

    #[test]
    fn emit_group_strips_lone_member_gap_when_flag_is_set() {
        let (source, members) = rows(&[(3, 5)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(8)).with_singleton_strip(),
            &mut edits,
        );

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
    fn no_shift_collapses_every_row_to_one_space() {
        let (source, members) = rows(&[(1, 3), (2, 3), (3, 3)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(MaxShift::NoShift),
            &mut edits,
        );

        // Every row stands alone, so each collapses to its one-space
        // suffix regardless of its neighbors' widths.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[0], 1),
                fill(&members[1], 1),
                fill(&members[2], 1)
            ],
        );
    }

    #[test]
    fn no_shift_keeps_equal_width_rows_flush() {
        let (source, members) = rows(&[(5, 3), (5, 3)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(MaxShift::NoShift).with_singleton_strip(),
            &mut edits,
        );

        // Equal widths would group under any positive cap, but NoShift
        // leaves each row its own singleton, so both strip flush rather
        // than taking the grouped one-space buffer.
        assert_eq!(
            sorted_summaries(&edits),
            vec![delete(&members[0]), delete(&members[1])],
        );
    }

    #[test]
    fn unlimited_folds_over_cap_spread_into_one_column() {
        let (source, members) = rows(&[(1, 1), (50, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(MaxShift::Unlimited),
            &mut edits,
        );

        // A 49-wide spread that would break under any cap folds into one
        // column aligned at the width-50 member.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 50)]);
    }

    #[test]
    fn walk_breaks_run_at_first_over_cap_row() {
        let (source, members) = rows(&[(1, 1), (2, 1), (3, 1), (15, 1)]);
        let mut edits = Vec::new();

        emit_group(&source, &members, Settings::aligned(cap(8)), &mut edits);

        // Widths 1/2/3 grow one group (spread 2), then width 15 pushes
        // the spread to 14 and breaks off as a natural singleton. The
        // leading group aligns at 3.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 3), fill(&members[1], 2)],
        );
    }

    #[test]
    fn walk_groups_in_source_order_not_by_width() {
        let (source, members) = rows(&[(1, 1), (2, 1), (15, 1), (3, 1), (4, 1)]);
        let mut edits = Vec::new();

        emit_group(&source, &members, Settings::aligned(cap(8)), &mut edits);

        // The width-15 row sits mid-run, so it breaks [1, 2] from [3, 4]
        // and stands alone rather than dragging the narrow rows into one
        // width band. [1, 2] aligns at 2 and [3, 4] aligns at 4.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 2), fill(&members[3], 2)],
        );
    }

    #[test]
    fn walk_keeps_row_at_exact_cap_boundary() {
        let (source, members) = rows(&[(1, 1), (9, 1)]);
        let mut edits = Vec::new();

        emit_group(&source, &members, Settings::aligned(cap(8)), &mut edits);

        // Spread 8 sits exactly at the cap, so the pair aligns at 9.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 9)]);
    }

    #[test]
    fn walk_leaves_over_cap_pair_natural() {
        let (source, members) = rows(&[(20, 1), (4, 1)]);
        let mut edits = Vec::new();

        emit_group(&source, &members, Settings::aligned(cap(8)), &mut edits);

        // Each member is its own group. Without strip, both singleton
        // targets are the one-space gap each row already carries.
        assert!(edits.is_empty());
    }

    #[test]
    fn walk_right_aligns_operators_within_a_group() {
        let (source, members) = rows(&[(12, 1), (11, 1), (1, 1)]);
        let members = [
            members[0].with_op_width(2),
            members[1].with_op_width(1),
            members[2].with_op_width(1),
        ];
        let mut edits = Vec::new();

        emit_group(&source, &members, Settings::aligned(cap(8)), &mut edits);

        // Widths 12/11 group and right-align on their widest operator, so
        // member[1] targets 1+1+1=3 spaces while member[0] keeps its
        // one-space gap. The width-1 row breaks off and takes no operator
        // padding from the wide group.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[1], 3)]);
    }

    #[test]
    fn walk_strips_a_singleton_broken_off_mid_run() {
        let (source, members) = rows(&[(20, 1), (2, 1), (3, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(8)).with_singleton_strip(),
            &mut edits,
        );

        // Width 20 breaks off first and strips to a zero-width gap, then
        // [2, 3] aligns at 3.
        assert_eq!(
            sorted_summaries(&edits),
            vec![delete(&members[0]), fill(&members[1], 2)],
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

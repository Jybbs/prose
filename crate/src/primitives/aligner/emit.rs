//! Padding-width math and edit emission for the alignment rules.
//! Splits each source-ordered run into the contiguous groups the
//! `max-shift` and governing line-length caps allow and rewrites each
//! member's gap to its group's column.

use ruff_diagnostics::Edit;
use ruff_source_file::LineRanges;
use ruff_text_size::TextRange;
use unicode_width::UnicodeWidthStr;

use super::{
    Member, Settings,
    holds::is_alignment_candidate,
    members::{baseline, line_gap_before},
};
use crate::{
    config::MaxShift,
    primitives::{
        comments::{TRAILING_GAP, trailing_comment_start},
        edit::repeat_edit,
    },
    source::Source,
};

/// Aligns `members` by splitting the source-ordered run into the
/// contiguous groups `reading_order_groups` yields and emitting each at
/// its widest member. A singleton group collapses its gap to the
/// settings' buffer, or to zero when `settings.strip_singleton` is set.
pub(super) fn emit_group(
    source: &Source,
    members: &[Member],
    settings: Settings,
    edits: &mut Vec<Edit>,
) {
    edits.extend(
        group_paddings(source, members, settings)
            .filter_map(|(m, pad)| space_padding_edit(source, m.gap, pad)),
    );
}

/// Per-member display column where each member's aligned token lands
/// under the same column math `emit_group` applies. A candidate group
/// reports its shared column, split on the `max-shift` cap, while any
/// other group reports the settings' buffer past each member's own
/// width.
/// The token's following value sits two columns further on, the
/// operator's final character plus the one-space value gap.
pub(crate) fn operator_columns(
    source: &Source,
    members: &[Member],
    settings: Settings,
) -> Vec<usize> {
    if !is_alignment_candidate(source, members) {
        return members
            .iter()
            .map(|m| baseline(source, *m) + m.width + settings.buffer)
            .collect();
    }
    group_paddings(source, members, settings)
        .map(|(m, pad)| baseline(source, m) + m.width + pad)
        .collect()
}

/// Returns the edit needed to make `range` carry exactly `n` ASCII
/// spaces, or `None` if it already does.
pub(crate) fn space_padding_edit(source: &Source, range: TextRange, n: usize) -> Option<Edit> {
    let text = source.slice(range);
    if text.len() == n && text.bytes().all(|b| b == b' ') {
        return None;
    }
    Some(repeat_edit(range, " ", n))
}

/// The width of `member`'s line as the aligner emits it, less the
/// pre-operator gap the padding replaces, with any rewritten
/// post-operator gap collapsed to the one space an emitted row carries,
/// and with a trailing comment measured at its [`TRAILING_GAP`] floor
/// rather than wherever the source leaves it.
fn emitted_base_width(source: &Source, member: Member) -> usize {
    let line = source.text().line_str(member.line_start).width();
    let base = line - source.slice(member.gap).width() - comment_slack(source, member);
    member
        .rewritten_value_gap(source)
        .map_or(base, |gap| base + 1 - source.slice(gap).width())
}

/// The columns the gap ahead of a trailing comment carries past
/// [`TRAILING_GAP`] on `member`'s line, zero where that line carries no
/// trailing comment and where the gap is the one `member` rewrites
/// itself.
fn comment_slack(source: &Source, member: Member) -> usize {
    let Some(comment) = trailing_comment_start(source, member.line_start) else {
        return 0;
    };
    let gap = line_gap_before(source, comment);
    if gap == member.gap {
        return 0;
    }
    source.slice(gap).width().saturating_sub(TRAILING_GAP.len())
}

/// True when no member of `group` aligned to `max_w` has its line
/// pushed past the governing line-length cap by the padding, and for a
/// rule carrying no cap at all. A member already over the cap unpadded
/// stays in the run rather than partitioning to no gain.
fn fits_line_cap(source: &Source, group: &[Member], settings: Settings, max_w: usize) -> bool {
    let Some(cap) = settings.line_length else {
        return true;
    };
    let max_op = max_op_width(group);
    group.iter().all(|m| {
        let base = emitted_base_width(source, *m);
        base + padding_width(*m, max_w, max_op, settings.buffer) <= cap
            || base + settings.buffer > cap
    })
}

/// True when `group` may align as one column: its width spread stays
/// within `shift_cap` and, when a `line_length` cap governs, every
/// member's aligned line stays within it.
fn group_holds(source: &Source, group: &[Member], settings: Settings, shift_cap: usize) -> bool {
    let max_w = group_max_width(group);
    let min_w = group.iter().map(|m| m.width).min().unwrap_or(0);
    max_w - min_w <= shift_cap && fits_line_cap(source, group, settings, max_w)
}

/// The widest member width in `group`, zero for an empty slice.
fn group_max_width(group: &[Member]) -> usize {
    group.iter().map(|m| m.width).max().unwrap_or(0)
}

/// Pairs every member with the gap width that lands its aligned token in
/// its group's shared column, walking the groups `reading_order_groups`
/// yields in source order.
fn group_paddings<'m>(
    source: &Source,
    members: &'m [Member],
    settings: Settings,
) -> impl Iterator<Item = (Member, usize)> + 'm {
    reading_order_groups(source, members, settings)
        .into_iter()
        .flat_map(move |(group, max_w)| {
            let suffix = settings.suffix_len(group.len());
            let max_op = max_op_width(group);
            group
                .iter()
                .map(move |m| (*m, padding_width(*m, max_w, max_op, suffix)))
        })
}

/// Returns the widest `op_width` in `members`, or `0` when the slice
/// is empty.
fn max_op_width(members: &[Member]) -> usize {
    members.iter().map(|m| m.op_width).max().unwrap_or(0)
}

/// The gap that lands `member`'s aligned token in its group's shared
/// column: a `suffix_len` buffer plus the slack to the widest member,
/// plus the operator slack that right-aligns variable-width operators on
/// their last character.
fn padding_width(member: Member, max_w: usize, max_op_w: usize, suffix_len: usize) -> usize {
    suffix_len + (max_w - member.width) + (max_op_w - member.op_width)
}

/// Splits the source-ordered `members` into the contiguous groups the
/// aligner emits independently, each paired with its widest member's
/// width. `Unlimited` gathers the whole run into one group, `NoShift`
/// leaves every row its own singleton, and `Cap(n)` grows a group while
/// its width spread stays within `n`. A governing `line_length` cap
/// grows a group only while every member's aligned line stays within it,
/// cutting a fresh group at the first row that would push the spread or a
/// line past its cap. Each group is a sub-slice, so a column never jumps
/// a row it skipped.
fn reading_order_groups<'m>(
    source: &Source,
    members: &'m [Member],
    settings: Settings,
) -> Vec<(&'m [Member], usize)> {
    let shift_cap = match settings.max_shift {
        MaxShift::NoShift => {
            return members
                .iter()
                .map(|m| (std::slice::from_ref(m), m.width))
                .collect();
        }
        MaxShift::Unlimited => usize::MAX,
        MaxShift::Cap(n) => n.get(),
    };
    if members.is_empty() {
        return Vec::new();
    }
    let mut groups = Vec::new();
    let mut start = 0;
    for i in 1..members.len() {
        if !group_holds(source, &members[start..=i], settings, shift_cap) {
            let prev = &members[start..i];
            groups.push((prev, group_max_width(prev)));
            start = i;
        }
    }
    let last = &members[start..];
    groups.push((last, group_max_width(last)));
    groups
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;
    use ruff_text_size::{Ranged, TextSize};

    use super::*;
    use crate::testing::{align_member, parse, range};

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

    /// Builds a two-row source whose `=` carries no space before it and
    /// `sep` between the operator and the value on the leading row,
    /// returning one width-1 and one width-6 `Member`, each with its
    /// pre- and post-operator gaps.
    fn paired_rows(sep: &str) -> (Source, Vec<Member>) {
        let head = format!("a={sep}");
        let first = format!("{head}11\n");
        let second = TextSize::of(&first);
        let members = vec![
            align_member(TextRange::empty(TextSize::new(1)), 0, 1)
                .with_value_gap(TextSize::of('='), TextSize::of(&head)),
            align_member(
                TextRange::empty(second + TextSize::new(6)),
                second.to_u32(),
                6,
            )
            .with_value_gap(TextSize::of('='), second + TextSize::new(7)),
        ];
        (parse(&format!("{first}abcdef=2\n")), members)
    }

    /// Builds a multi-line Python source where each row is
    /// `x...x{spaces}= 0\n`, returns the source plus one `Member` per
    /// row pointing at that row's pre-`=` whitespace. `gap_chars` seeds
    /// the existing pre-`=` whitespace.
    fn rows(specs: &[(usize, usize)]) -> (Source, Vec<Member>) {
        let mut text = String::new();
        let mut members = Vec::new();
        for &(width, gap_chars) in specs {
            let line_start = TextSize::of(&text);
            text.push_str(&"x".repeat(width));
            let gap_start = TextSize::of(&text);
            text.push_str(&" ".repeat(gap_chars));
            let gap_end = TextSize::of(&text);
            text.push_str("= 0\n");
            members.push(align_member(
                TextRange::new(gap_start, gap_end),
                line_start.to_u32(),
                width,
            ));
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
    fn emit_group_seats_a_widened_buffer_ahead_of_the_token() {
        let (source, members) = rows(&[(1, 1), (3, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(8)).with_buffer(2),
            &mut edits,
        );

        // max_w=3, buffer=2 → targets 4 and 2 spaces.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 4), fill(&members[1], 2)],
        );
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
    fn line_cap_counts_the_post_operator_space() {
        let (source, members) = paired_rows("");
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(16)).with_line_length(10),
            &mut edits,
        );

        // Padding the width-1 row to the width-6 name lands its line on
        // the cap only while the space after `=` goes uncounted, so the
        // run declines the shared column and each row buffers by one.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 1), fill(&members[1], 1)],
        );
    }

    #[test]
    fn line_cap_discounts_a_value_gap_that_crosses_a_line_break() {
        let (source, members) = paired_rows("\\\n    ");
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(16)).with_line_length(10),
            &mut edits,
        );

        // The continued row's value opens on a later line, so its gap is
        // measured as written rather than collapsed, leaving the pair
        // inside the cap and sharing a column.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 6), fill(&members[1], 1)],
        );
    }

    #[rstest]
    fn line_cap_holds_the_run_when_both_spaces_fit(#[values("", "   ")] sep: &str) {
        let (source, members) = paired_rows(sep);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(16)).with_line_length(11),
            &mut edits,
        );

        // One column of headroom past the declining case covers both
        // inserted spaces. The three-space variant collapses to that same
        // one space when emitted, so its existing gap is discounted
        // rather than counted on top of the inserted one.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 6), fill(&members[1], 1)],
        );
    }

    #[test]
    fn line_cap_measures_a_trailing_comment_at_its_floor() {
        // `align_comments` seats the comment two columns past the code
        // downstream, so the sixteen-column gutter the source carries
        // never counts against this run's budget.
        let source = parse("a = 1                # note\nabcdefg = 2  # note\n");
        let members = [
            align_member(range(1, 2), 0, 1),
            align_member(range(35, 36), 28, 7),
        ];
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(8)).with_line_length(28),
            &mut edits,
        );

        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 7)]);
    }

    #[test]
    fn line_cap_keeps_a_row_already_past_the_cap_in_its_run() {
        let (source, members) = rows(&[(12, 1), (13, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            Settings::aligned(cap(16)).with_line_length(8),
            &mut edits,
        );

        // Both lines sit past the cap before any padding, so partitioning
        // buys nothing and the pair still aligns at width 13.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 2)]);
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
    fn operator_columns_aligns_a_candidate_group_to_one_column() {
        let (source, members) = rows(&[(1, 1), (2, 1), (3, 1)]);

        // Three distinct-line rows at one baseline align their operator to
        // the widest member, so every column lands one past width 3.
        assert_eq!(
            operator_columns(&source, &members, Settings::aligned(cap(8))),
            vec![4, 4, 4],
        );
    }

    #[test]
    fn operator_columns_buffers_a_lone_member_past_its_width() {
        let (source, members) = rows(&[(3, 5)]);

        // A singleton is no candidate, so its operator takes a one-space
        // buffer past the width-3 name rather than a shared column.
        assert_eq!(
            operator_columns(&source, &members, Settings::aligned(cap(8))),
            vec![4],
        );
    }

    #[test]
    fn operator_columns_splits_a_candidate_run_on_max_shift() {
        let (source, members) = rows(&[(1, 1), (15, 1)]);

        // The width-14 spread breaks the cap, so each row forms its own
        // column: the narrow operator at 2, the wide one at 16.
        assert_eq!(
            operator_columns(&source, &members, Settings::aligned(cap(8))),
            vec![2, 16],
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
}

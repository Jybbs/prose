//! Padding-width math and edit emission for the alignment rules.
//! Splits each source-ordered run into the contiguous groups the
//! `max-shift` and governing line-length caps allow and rewrites each
//! member's gap to its group's column.

use std::ops::RangeInclusive;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_source_file::LineRanges;
use ruff_text_size::{TextRange, TextSize};

use super::{
    Cap, Member, Settings, Widenings, holds::is_alignment_candidate, members::line_gap_before,
};
use crate::{
    config::MaxShift,
    primitives::{
        comments::{Settling, trailing_comment},
        edit::repeat_edit,
        inline::display_width,
        padding::{self, Stranding},
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
    widenings: &Widenings,
    edits: &mut Vec<Edit>,
) {
    edits.extend(
        group_paddings(source, members, settings, widenings, &[])
            .filter_map(|(m, pad)| space_padding_edit(source, m.gap, pad)),
    );
}

/// [`operator_columns`] for the rows a rule writes, a row `joined`
/// names standing on a line of its own where the source seats it on
/// its neighbor's, whereas a value merely widening an existing row
/// stands no run up.
pub(crate) fn forecast_columns(
    source: &Source,
    members: &[Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
) -> Vec<usize> {
    columns(
        source,
        members,
        settings,
        widenings,
        joined,
        is_forecast_candidate(members, joined),
    )
}

/// Per-member display column where each member's aligned token lands
/// under `emit_group`'s column math, each line read with `widenings`
/// and at the width `joined` names where a later rule writes the row.
/// A candidate group reports its shared column, any other group the
/// settings' buffer past each member's width, and the value follows
/// [`VALUE_OFFSET`](super::VALUE_OFFSET) columns on.
pub(crate) fn operator_columns(
    source: &Source,
    members: &[Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
) -> Vec<usize> {
    columns(
        source,
        members,
        settings,
        widenings,
        joined,
        is_alignment_candidate(members),
    )
}

/// The columns `member`'s line keeps past `code_end` once the comment
/// rules `settings` carries settle the trailing comment there, the
/// written tail where no cap governs.
pub(crate) fn settled_tail(
    source: &Source,
    member: Member,
    settings: Settings,
    code_end: TextSize,
) -> usize {
    let tail = display_width(source.slice(source.row_tail(code_end)));
    settings.cap.map_or(tail, |cap| {
        tail.saturating_add_signed(-comment_slack(source, member, cap.settling))
    })
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

/// The per-member columns of a run, the group math where `candidate`
/// holds and the settings' buffer past each member's own width
/// otherwise.
fn columns(
    source: &Source,
    members: &[Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
    candidate: bool,
) -> Vec<usize> {
    if !candidate {
        return members
            .iter()
            .map(|m| m.baseline + m.settled_width + settings.buffer)
            .collect();
    }
    group_paddings(source, members, settings, widenings, joined)
        .map(|(m, pad)| m.baseline + m.settled_width + pad)
        .collect()
}

/// The columns a trailing comment on `member`'s line carries away from
/// the width `settling`'s comment rules leave it at, zero where the
/// line carries no trailing comment, per [`Settling::slack`] with the
/// gap `member` rewrites itself left out.
fn comment_slack(source: &Source, member: Member, settling: Settling) -> isize {
    trailing_comment(source, member.line_start).map_or(0, |comment| {
        let gap = line_gap_before(source, comment.start());
        settling.slack(source, comment, gap, member.gap)
    })
}

/// The width of `member`'s line as the aligner emits it: less the
/// pre-operator gap the padding replaces, any rewritten post-operator
/// gap collapsed to one space, a trailing comment at the gap and opener
/// widths `cap`'s comment rules settle it to, and the padding its
/// padding rule drops elsewhere on the line gone. A `joined` width
/// stands in for the line as written where a later rule joins the
/// member's value onto it, the code past the value then reading as
/// that join leaves it and the padding inside the value already gone.
fn emitted_base_width(source: &Source, member: Member, cap: Cap, joined: Option<usize>) -> usize {
    let line = source.text().line_range(member.line_start);
    let (written, padded) = match joined {
        Some(width) => (width, TextRange::new(line.start(), member.gap.end())),
        None => (display_width(source.slice(line)), line),
    };
    let slack = joined.map_or_else(|| comment_slack(source, member, cap.settling), |_| 0)
        + padding_slack(source, member, padded, cap.stranding);
    let base = (written - display_width(source.slice(member.gap))).saturating_add_signed(-slack);
    member
        .rewritten_value_gap(source)
        .map_or(base, |gap| base + 1 - display_width(source.slice(gap)))
}

/// Each member's emitted line width, measured once per run rather than
/// once per group-growth step. Empty where no line-length cap governs,
/// since [`fits_line_cap`] then reads none of them.
fn emitted_bases(
    source: &Source,
    members: &[Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
) -> Vec<usize> {
    let Some(cap) = settings.cap else {
        return Vec::new();
    };
    members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            emitted_base_width(source, *m, cap, joined.get(i).copied().flatten())
                .saturating_add_signed(widenings.delta(*m))
        })
        .collect()
}

/// True when no member of `group` aligned to `max_w` has its line
/// pushed past the governing line-length cap by the padding, and for a
/// rule carrying no cap at all. `bases` carries each member's emitted
/// width in step with `group`. A member over the cap even at its
/// singleton fallback gap stays in the run only where the shared column
/// costs it no further width than the buffer, which holds for the
/// widest member alone, so aligning never carries an over-cap line
/// further out and never pushes a fitting line past the cap.
fn fits_line_cap(group: &[Member], bases: &[usize], settings: Settings, max_w: usize) -> bool {
    let Some(cap) = settings.cap.map(|cap| cap.line_length) else {
        return true;
    };
    let max_op = max_op_width(group);
    group.iter().zip(bases).all(|(m, base)| {
        let padding = padding_width(*m, max_w, max_op, settings.buffer);
        base + padding <= cap || (padding == settings.buffer && base + settings.suffix_len(1) > cap)
    })
}

/// True when `group` may align as one column: its settled-width spread
/// stays within `shift_cap` and, when a `line_length` cap governs,
/// every member's aligned line stays within it.
fn group_holds(group: &[Member], bases: &[usize], settings: Settings, shift_cap: usize) -> bool {
    let (min_w, max_w) = group
        .iter()
        .map(|m| m.settled_width)
        .minmax()
        .into_option()
        .unwrap_or((0, 0));
    max_w - min_w <= shift_cap && fits_line_cap(group, bases, settings, max_w)
}

/// The widest settled width in `group`, zero for an empty slice.
fn group_max_width(group: &[Member]) -> usize {
    group.iter().map(|m| m.settled_width).max().unwrap_or(0)
}

/// Pairs every member with the gap width that lands its aligned token in
/// its group's shared column, walking the groups `reading_order_groups`
/// yields in source order.
fn group_paddings<'m>(
    source: &Source,
    members: &'m [Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
) -> impl Iterator<Item = (Member, usize)> + 'm {
    reading_order_groups(source, members, settings, widenings, joined)
        .into_iter()
        .flat_map(move |(group, max_w)| {
            let suffix = settings.suffix_len(group.len());
            let max_op = max_op_width(group);
            group
                .iter()
                .map(move |m| (*m, padding_width(*m, max_w, max_op, suffix)))
        })
}

/// [`is_alignment_candidate`] with a pair excused where `joined` names
/// the later row as one a rule writes, which then stands on a line of
/// its own beneath the row before it.
fn is_forecast_candidate(members: &[Member], joined: &[Option<usize>]) -> bool {
    members.len() >= 2
        && members.windows(2).enumerate().all(|(i, pair)| {
            pair[0].baseline == pair[1].baseline
                && (pair[0].line_start != pair[1].line_start
                    || joined.get(i + 1).is_some_and(Option::is_some))
        })
}

/// Returns the widest `op_width` in `members`, or `0` when the slice
/// is empty.
fn max_op_width(members: &[Member]) -> usize {
    members.iter().map(|m| m.op_width).max().unwrap_or(0)
}

/// The columns the padding rule `stranding` names takes off `member`'s
/// `line`, leaving aside the gaps the aligner rewrites itself.
fn padding_slack(source: &Source, member: Member, line: TextRange, stranding: Stranding) -> isize {
    let edits = source.stranded_padding(stranding);
    let own = member
        .rewritten_value_gap(source)
        .map_or(0, |gap| padding::slack(source, &edits, gap));
    padding::slack(source, &edits, line) - padding::slack(source, &edits, member.gap) - own
}

/// The gap that lands `member`'s aligned token in its group's shared
/// column: a `suffix_len` buffer plus the slack to the group's widest
/// settled width, plus the operator slack that right-aligns
/// variable-width operators on their last character.
fn padding_width(member: Member, max_w: usize, max_op_w: usize, suffix_len: usize) -> usize {
    suffix_len + (max_w - member.settled_width) + (max_op_w - member.op_width)
}

/// Splits the source-ordered `members` into the contiguous groups the
/// aligner emits independently, each paired with its widest settled
/// width. `Unlimited` gathers the whole run, `NoShift` leaves every
/// row its own singleton, and `Cap(n)` grows a group while its spread
/// stays within `n` and, under a governing line cap, while every
/// aligned line fits. Under `release_heads`, a head only the line cap
/// pins releases as a singleton so the cut row joins the column
/// beneath it. Each group is a sub-slice, so a column never jumps a
/// row it skipped.
fn reading_order_groups<'m>(
    source: &Source,
    members: &'m [Member],
    settings: Settings,
    widenings: &Widenings,
    joined: &[Option<usize>],
) -> Vec<(&'m [Member], usize)> {
    let shift_cap = match settings.max_shift {
        MaxShift::NoShift => {
            return members
                .iter()
                .map(|m| (std::slice::from_ref(m), m.settled_width))
                .collect();
        }
        MaxShift::Unlimited => usize::MAX,
        MaxShift::Cap(n) => n.get(),
    };
    if members.is_empty() {
        return Vec::new();
    }
    let bases = emitted_bases(source, members, settings, widenings, joined);
    let holds = |range: RangeInclusive<usize>| {
        group_holds(
            &members[range.clone()],
            bases.get(range).unwrap_or_default(),
            settings,
            shift_cap,
        )
    };
    let pinned = |range: RangeInclusive<usize>| {
        let head = *range.start();
        !fits_line_cap(
            &members[head..=head],
            bases.get(head..=head).unwrap_or_default(),
            settings,
            group_max_width(&members[range]),
        )
    };
    let mut groups = Vec::new();
    let mut start = 0;
    let mut i = 1;
    while i < members.len() {
        if holds(start..=i) {
            i += 1;
            continue;
        }
        let released = settings.release_heads
            && i - start >= 2
            && pinned(start..=i)
            && (i + 1 == members.len() || !holds(i..=i + 1))
            && holds(start + 1..=i);
        let cut = if released { start + 1 } else { i };
        let prev = &members[start..cut];
        groups.push((prev, group_max_width(prev)));
        start = cut;
        i = i.max(cut + 1);
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
    use crate::{
        rule::RuleId,
        testing::{align_member, parse, range},
    };

    /// The padding rule every capped setting here reads, over sources
    /// carrying no padding.
    fn strip() -> Stranding {
        Stranding::new(RuleId::from("strip-stranded-padding"), true)
    }

    /// The comment rules every capped setting here reads, both on.
    fn settling() -> Settling {
        Settling {
            gap: Some(RuleId::from("align-comments")),
            opener: Some(RuleId::from("normalize-comment-spacing")),
        }
    }

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

    /// The edits `emit_group` writes for `members` under `settings` with
    /// no widenings.
    fn emitted(source: &Source, members: &[Member], settings: Settings) -> Vec<Edit> {
        let mut edits = Vec::new();
        emit_group(source, members, settings, &Widenings::default(), &mut edits);
        edits
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
            text.extend(std::iter::repeat_n(' ', gap_chars));
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

    /// Builds a multi-line Python source where each row is
    /// `x...x = {tail}\n` with a one-space gap before `=`, returning the
    /// source plus one `Member` per row pointing at that gap.
    fn tailed_rows(specs: &[(usize, &str)]) -> (Source, Vec<Member>) {
        let mut text = String::new();
        let mut members = Vec::new();
        for &(width, tail) in specs {
            let line_start = TextSize::of(&text);
            text.push_str(&"x".repeat(width));
            let gap_start = TextSize::of(&text);
            text.push(' ');
            let gap_end = TextSize::of(&text);
            text.push_str(&format!("= {tail}\n"));
            members.push(align_member(
                TextRange::new(gap_start, gap_end),
                line_start.to_u32(),
                width,
            ));
        }
        (parse(&text), members)
    }

    fn sorted_summaries(edits: &[Edit]) -> Vec<(u32, u32, String)> {
        edits.iter().map(summary).sorted().collect()
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
        let edits = emitted(&source, &members, Settings::aligned(cap(10)));

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
        let edits = emitted(&source, &members, Settings::aligned(cap(8)));

        // single member fits any cap. max_w=3, padding=0, suffix=1 →
        // target 1 space, currently 5.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 1)]);
    }

    #[test]
    fn emit_group_handles_empty_member_slice() {
        let source = parse("x = 0\n");
        let edits = emitted(&source, &[], Settings::aligned(cap(8)));

        assert!(edits.is_empty());
    }

    #[test]
    fn emit_group_seats_a_widened_buffer_ahead_of_the_token() {
        let (source, members) = rows(&[(1, 1), (3, 1)]);
        let edits = emitted(&source, &members, Settings::aligned(cap(8)).with_buffer(2));

        // max_w=3, buffer=2 → targets 4 and 2 spaces.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 4), fill(&members[1], 2)],
        );
    }

    #[test]
    fn emit_group_strips_lone_member_gap_when_flag_is_set() {
        let (source, members) = rows(&[(3, 5)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(8)).with_singleton_strip(),
        );

        // A lone member is its own group, so strip collapses the
        // five-space gap to zero rather than the one-space suffix.
        assert_eq!(sorted_summaries(&edits), vec![delete(&members[0])]);
    }

    #[test]
    fn forecast_columns_seats_a_written_row_beneath_the_row_before() {
        // Two members sharing one source line, the later a row a rule
        // writes, so the run stands as a column at the widest width.
        let source = parse("a = 1; bcd = 2\n");
        let members = [
            align_member(range(2, 3), 0, 2),
            align_member(range(4, 5), 0, 4),
        ];
        let joined = [None, Some(12)];

        assert_eq!(
            forecast_columns(
                &source,
                &members,
                Settings::aligned(cap(8)),
                &Widenings::default(),
                &joined,
            ),
            vec![5, 5],
        );
    }

    #[test]
    fn line_cap_counts_the_post_operator_space() {
        let (source, members) = paired_rows("");
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16)).within(10, strip(), settling()),
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
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16)).within(10, strip(), settling()),
        );

        // The continued row's value opens on a later line, so its gap is
        // measured as written rather than collapsed, leaving the pair
        // inside the cap and sharing a column.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 6), fill(&members[1], 1)],
        );
    }

    #[test]
    fn line_cap_holds_an_over_cap_row_costing_no_further_width() {
        let (source, members) = rows(&[(13, 1), (13, 5)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16)).within(8, strip(), settling()),
        );

        // Equal widths leave the column at each row's own buffer, so the
        // over-cap pair aligns rather than partitioning to no gain.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[1], 1)]);
    }

    #[test]
    fn line_cap_holds_an_over_cap_row_out_of_a_widening_column() {
        let (source, members) = rows(&[(12, 1), (13, 1)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16)).within(8, strip(), settling()),
        );

        // Both lines sit past the cap unpadded, and the shared column
        // would carry the narrow row one further out, so the run splits
        // and each row keeps the buffer it already holds.
        assert!(edits.is_empty());
    }

    #[rstest]
    fn line_cap_holds_the_run_when_both_spaces_fit(#[values("", "   ")] sep: &str) {
        let (source, members) = paired_rows(sep);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16)).within(11, strip(), settling()),
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
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(8)).within(28, strip(), settling()),
        );

        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 7)]);
    }

    #[test]
    fn no_shift_collapses_every_row_to_one_space() {
        let (source, members) = rows(&[(1, 3), (2, 3), (3, 3)]);
        let edits = emitted(&source, &members, Settings::aligned(MaxShift::NoShift));

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
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(MaxShift::NoShift).with_singleton_strip(),
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
            operator_columns(
                &source,
                &members,
                Settings::aligned(cap(8)),
                &Widenings::default(),
                &[],
            ),
            vec![4, 4, 4],
        );
    }

    #[test]
    fn operator_columns_buffers_a_lone_member_past_its_width() {
        let (source, members) = rows(&[(3, 5)]);

        // A singleton is no candidate, so its operator takes a one-space
        // buffer past the width-3 name rather than a shared column.
        assert_eq!(
            operator_columns(
                &source,
                &members,
                Settings::aligned(cap(8)),
                &Widenings::default(),
                &[],
            ),
            vec![4],
        );
    }

    #[test]
    fn operator_columns_keeps_same_line_members_at_their_own_columns() {
        // The same pair as the forecast test, read as the source wrote
        // it, so a value joined onto an existing row stands no run up.
        let source = parse("a = 1; bcd = 2\n");
        let members = [
            align_member(range(2, 3), 0, 2),
            align_member(range(4, 5), 0, 4),
        ];
        let joined = [None, Some(12)];

        assert_eq!(
            operator_columns(
                &source,
                &members,
                Settings::aligned(cap(8)),
                &Widenings::default(),
                &joined,
            ),
            vec![3, 5],
        );
    }

    #[test]
    fn operator_columns_splits_a_candidate_run_on_max_shift() {
        let (source, members) = rows(&[(1, 1), (15, 1)]);

        // The width-14 spread breaks the cap, so each row forms its own
        // column: the narrow operator at 2, the wide one at 16.
        assert_eq!(
            operator_columns(
                &source,
                &members,
                Settings::aligned(cap(8)),
                &Widenings::default(),
                &[],
            ),
            vec![2, 16],
        );
    }

    #[test]
    fn settled_tail_reads_the_comment_at_the_width_the_comment_rules_leave() {
        let source = parse("a = 1    # note\n");
        let member = align_member(range(1, 2), 0, 1);
        let uncapped = Settings::aligned(cap(8));
        let capped = uncapped.within(20, strip(), settling());

        // The written tail past the value holds four spaces and the
        // comment, which the gap rule settles to the two-space floor.
        assert_eq!(
            settled_tail(&source, member, uncapped, TextSize::new(5)),
            10
        );
        assert_eq!(settled_tail(&source, member, capped, TextSize::new(5)), 8);
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
        let edits = emitted(&source, &members, Settings::aligned(MaxShift::Unlimited));

        // A 49-wide spread that would break under any cap folds into one
        // column aligned at the width-50 member.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 50)]);
    }

    #[test]
    fn walk_advances_past_a_singleton_holding_as_no_group() {
        let (source, members) = rows(&[(5, 1), (12, 1)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16))
                .with_singleton_strip()
                .within(15, strip(), settling()),
        );

        // The width-12 row lands on the cap stripped and one past it
        // buffered, so it holds as no group even alone, and the walk
        // steps past it rather than re-testing it as its own group.
        assert_eq!(
            sorted_summaries(&edits),
            vec![delete(&members[0]), delete(&members[1])],
        );
    }

    #[test]
    fn walk_breaks_run_at_first_over_cap_row() {
        let (source, members) = rows(&[(1, 1), (2, 1), (3, 1), (15, 1)]);
        let edits = emitted(&source, &members, Settings::aligned(cap(8)));

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
        let edits = emitted(&source, &members, Settings::aligned(cap(8)));

        // The width-15 row sits mid-run, so it breaks [1, 2] from [3, 4]
        // and stands alone rather than dragging the narrow rows into one
        // width band. [1, 2] aligns at 2 and [3, 4] aligns at 4.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 2), fill(&members[3], 2)],
        );
    }

    #[test]
    fn walk_keeps_a_head_no_line_cap_pins() {
        let (source, members) = rows(&[(1, 1), (8, 1), (9, 1), (10, 1)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(8)).releasing_heads(),
        );

        // Width 10 breaks 1/8/9 on the spread alone and would stand
        // alone, but only a line cap pins a head, so the prefix stays as
        // cut rather than trading its column for the cut row's.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 9), fill(&members[1], 2)],
        );
    }

    #[test]
    fn walk_keeps_a_head_the_spread_alone_cuts() {
        let (source, members) = rows(&[(1, 1), (8, 1), (9, 1), (10, 1)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(8))
                .within(40, strip(), settling())
                .releasing_heads(),
        );

        // Width 10 breaks 1/8/9 on the spread while every line sits far
        // inside the cap, so the head could follow the cut row's column
        // and stays the prefix's head, leaving width 10 a singleton.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 9), fill(&members[1], 2)],
        );
    }

    #[test]
    fn walk_keeps_a_pinned_head_where_the_rule_releases_none() {
        let (source, members) = tailed_rows(&[(3, "123456"), (1, "0"), (2, "0"), (6, "0")]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16)).within(12, strip(), settling()),
        );

        // The same pinned head stays the prefix's head for a rule that
        // releases none, so width 6 breaks off as a singleton.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[1], 3), fill(&members[2], 2)],
        );
    }

    #[test]
    fn walk_keeps_row_at_exact_cap_boundary() {
        let (source, members) = rows(&[(1, 1), (9, 1)]);
        let edits = emitted(&source, &members, Settings::aligned(cap(8)));

        // Spread 8 sits exactly at the cap, so the pair aligns at 9.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[0], 9)]);
    }

    #[test]
    fn walk_keeps_the_head_when_the_cut_row_pairs_beneath() {
        let (source, members) =
            tailed_rows(&[(3, "123456"), (1, "0"), (2, "0"), (6, "0"), (5, "0")]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16))
                .within(12, strip(), settling())
                .releasing_heads(),
        );

        // Width 6 breaks the pinned head's group but pairs with width 5,
        // so the cut row stands alone nowhere and the head keeps its
        // group.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[1], 3),
                fill(&members[2], 2),
                fill(&members[4], 2)
            ],
        );
    }

    #[test]
    fn walk_leaves_over_cap_pair_natural() {
        let (source, members) = rows(&[(20, 1), (4, 1)]);
        let edits = emitted(&source, &members, Settings::aligned(cap(8)));

        // Each member is its own group. Without strip, both singleton
        // targets are the one-space gap each row already carries.
        assert!(edits.is_empty());
    }

    #[test]
    fn walk_releases_a_cap_pinned_head_that_would_strand_the_cut_row() {
        let (source, members) = tailed_rows(&[(3, "123456"), (1, "0"), (2, "0"), (6, "0")]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(16))
                .within(12, strip(), settling())
                .releasing_heads(),
        );

        // The column the width-6 row sets would carry the width-3 head
        // past the cap, cutting width 6 off as a singleton. Releasing
        // the head instead lets 1/2/6 share the column at 6.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[1], 6), fill(&members[2], 5)],
        );
    }

    #[test]
    fn walk_right_aligns_operators_within_a_group() {
        let (source, members) = rows(&[(12, 1), (11, 1), (1, 1)]);
        let members = [
            members[0].with_op_width(2),
            members[1].with_op_width(1),
            members[2].with_op_width(1),
        ];
        let edits = emitted(&source, &members, Settings::aligned(cap(8)));

        // Widths 12/11 group and right-align on their widest operator, so
        // member[1] targets 1+1+1=3 spaces while member[0] keeps its
        // one-space gap. The width-1 row breaks off and takes no operator
        // padding from the wide group.
        assert_eq!(sorted_summaries(&edits), vec![fill(&members[1], 3)]);
    }

    #[test]
    fn walk_strips_a_singleton_broken_off_mid_run() {
        let (source, members) = rows(&[(20, 1), (2, 1), (3, 1)]);
        let edits = emitted(
            &source,
            &members,
            Settings::aligned(cap(8)).with_singleton_strip(),
        );

        // Width 20 breaks off first and strips to a zero-width gap, then
        // [2, 3] aligns at 3.
        assert_eq!(
            sorted_summaries(&edits),
            vec![delete(&members[0]), fill(&members[1], 2)],
        );
    }
}

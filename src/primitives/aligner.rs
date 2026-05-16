//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. `emit_group` is the
//! entry point. Per-rule knobs travel through `Settings`. Aligned
//! rows always carry a one-space buffer between content and the
//! aligned token.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::{AnyParameterRef, Parameters, Stmt};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::config::{AlignmentConfig, MaxAlignShiftPolicy};
use crate::source::Source;

/// One row in an alignment group.
///
/// `width` is the display-column width of the row's left-hand-side
/// region, from the start of the member to the start of the gap. `gap`
/// is the whitespace range ending immediately before the aligned
/// token that the rule will rewrite. `line_start` is the offset of
/// the start of the source line containing the gap.
#[derive(Clone, Copy)]
pub(crate) struct Member {
    pub gap: TextRange,
    pub line_start: TextSize,
    pub width: usize,
}

/// Emission knobs shared by every alignment rule.
///
/// `strip_singleton_subgroup` collapses size-one sub-groups in
/// `emit_split` to a zero-width gap, except when the singleton is
/// strictly the widest member of the group, in which case it anchors
/// every sub-group's padding at `singleton.width + 1`.
#[derive(Clone, Copy)]
pub(crate) struct Settings {
    max_shift: usize,
    policy: MaxAlignShiftPolicy,
    strip_singleton_subgroup: bool,
}

impl Settings {
    /// Returns a copy of `self` with `strip_singleton_subgroup` enabled.
    pub(crate) fn with_singleton_subgroup_strip(mut self) -> Self {
        self.strip_singleton_subgroup = true;
        self
    }
}

impl From<&AlignmentConfig> for Settings {
    fn from(c: &AlignmentConfig) -> Self {
        Self {
            max_shift: c.max_shift.get(),
            policy: c.max_shift_policy,
            strip_singleton_subgroup: false,
        }
    }
}

/// Aligns the members of a group, dispatching through `settings.policy`
/// when the widest padding exceeds `settings.max_shift`. A `Split`
/// sub-group of size one collapses its gap to one space, or to zero
/// when `settings.strip_singleton_subgroup` is set.
pub(crate) fn emit_group(
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
    if max_w - min_w <= settings.max_shift {
        emit_with_paddings(source, members, max_w, 1, edits);
        return;
    }
    match settings.policy {
        MaxAlignShiftPolicy::Drop => emit_drop(source, members, settings, edits),
        MaxAlignShiftPolicy::Skip => {}
        MaxAlignShiftPolicy::Split => emit_split(source, members, settings, edits),
    }
}

/// Returns `true` when `members` form a multi-row group whose
/// aligned tokens sit on distinct source lines.
pub(crate) fn is_alignment_candidate(members: &[Member]) -> bool {
    members.len() >= 2
        && members
            .windows(2)
            .all(|w| w[0].line_start != w[1].line_start)
}

/// Generalization of [`line_adjacent_groups`] for rules that admit
/// more than one member shape. The qualifier returns `Option<(K, M)>`
/// where `K` tags the shape, and a run extends only while the next
/// member shares the active key, sits line-adjacent to the prior
/// statement, and the prior statement itself fits on one source line.
/// A key change at an otherwise-adjacent boundary closes the active
/// run and starts a fresh one without losing the boundary statement.
/// Walks `body` exactly once, calling the qualifier and each boundary
/// predicate at most once per statement.
pub(crate) fn keyed_line_adjacent_groups<'a, K, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    mut qualify: F,
) -> Vec<Vec<M>>
where
    K: Eq,
    F: FnMut(&'a Stmt) -> Option<(K, M)>,
{
    let mut groups: Vec<Vec<M>> = Vec::new();
    let mut active: Option<(K, TextRange)> = None;
    for stmt in body {
        let Some((key, member)) = qualify(stmt) else {
            active = None;
            continue;
        };
        let extends = active.as_ref().is_some_and(|(active_key, prev)| {
            active_key == &key
                && !source.contains_line_break(prev)
                && source.is_line_adjacent(TextRange::new(prev.end(), stmt.start()))
        });
        if extends {
            groups
                .last_mut()
                .expect("active implies groups non-empty")
                .push(member);
        } else {
            groups.push(vec![member]);
        }
        active = Some((key, stmt.range()));
    }
    groups
}

/// Walks `body`, qualifying each statement through `qualify` and
/// grouping the qualified members into runs where every consecutive
/// pair sits on adjacent source lines. A multi-line prior statement,
/// a non-qualifying statement, a comment in the inter-statement gap,
/// or a blank line breaks the current run. Empty groups (statements
/// that fail qualification with no qualified neighbors) are skipped.
/// Thin wrapper over [`keyed_line_adjacent_groups`] for rules whose
/// qualifier produces only one form, so every member shares an
/// implicit `()` key.
pub(crate) fn line_adjacent_groups<'a, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    mut qualify: F,
) -> Vec<Vec<M>>
where
    F: FnMut(&'a Stmt) -> Option<M>,
{
    keyed_line_adjacent_groups(source, body, move |stmt| qualify(stmt).map(|m| ((), m)))
}

/// Builds a `Member` for a row whose aligned token sits at `anchor`.
/// Width is the display width of the line's content from the first
/// non-whitespace character to the last non-whitespace character
/// before the gap, leaving the gap free for the rule to rewrite.
pub(crate) fn line_anchored_member(source: &Source, anchor: TextSize) -> Member {
    let line_start = source.text().line_start(anchor);
    let prefix = source.slice(TextRange::new(line_start, anchor));
    let trimmed_end = prefix.trim_whitespace_end();
    let gap_start = line_start + TextSize::of(trimmed_end);
    Member {
        gap: TextRange::new(gap_start, anchor),
        line_start,
        width: trimmed_end.trim_whitespace_start().width(),
    }
}

/// Builds a `Member` whose anchor is the first token of `kind` within
/// `search`. Returns `None` when the search turns up nothing.
pub(crate) fn line_anchored_member_at_kind(
    source: &Source,
    search: TextRange,
    kind: TokenKind,
) -> Option<Member> {
    let anchor = source.first_token_offset_in_range(search, |t| t.kind() == kind)?;
    Some(line_anchored_member(source, anchor))
}

/// Walks `params` in source order, qualifying each parameter through
/// `qualify` and returning one group per run of contiguous qualified
/// parameters. A parameter that fails to qualify breaks the current
/// run without joining either neighbor. Empty runs are filtered out.
pub(crate) fn parameter_split_groups<F>(params: &Parameters, qualify: F) -> Vec<Vec<Member>>
where
    F: FnMut(AnyParameterRef<'_>) -> Option<Member>,
{
    let qualified: Vec<_> = params.iter_source_order().map(qualify).collect();
    qualified
        .split(Option::is_none)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| chunk.iter().copied().flatten().collect())
        .collect()
}

/// Builds a `Member` whose anchor is the first token in `search`
/// satisfying `predicate`, with width measured by `target` plus
/// `extra_width`. Returns `None` if no token matches, or if the span
/// from `target.start()` to the anchor crosses a newline (continuation
/// imports, line-broken assignments). Use this when alignment must
/// stay confined to a single source line.
pub(crate) fn range_anchored_member_single_line<F>(
    source: &Source,
    target: TextRange,
    search: TextRange,
    predicate: F,
    extra_width: usize,
) -> Option<Member>
where
    F: FnMut(&Token) -> bool,
{
    let anchor = source.first_token_offset_in_range(search, predicate)?;
    if source.contains_line_break(TextRange::new(target.start(), anchor)) {
        return None;
    }
    Some(range_anchored_member(source, target, anchor, extra_width))
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
    emit_with_paddings(source, kept, max_w, 1, edits);
}

/// Partitions greedily into sub-groups capped at `settings.max_shift`
/// spread, each aligning at its own widest member by default. A
/// singleton collapses its gap to one space, or to zero when
/// `settings.strip_singleton_subgroup` is set. The strip shortcut
/// inverts when the singleton is strictly the widest member, wherein
/// it anchors every sub-group at `singleton.width + 1`.
fn emit_split(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
    let subs = partition_by_spread(members, settings.max_shift);
    let anchor = settings
        .strip_singleton_subgroup
        .then(|| widest_singleton_anchor_width(members, &subs))
        .flatten();
    for &(start, end, sub_max_w) in &subs {
        let sub = &members[start..end];
        let (max_w, suffix) = match (anchor, sub.len()) {
            (Some(a), _) => (a, 1),
            (None, 1) if settings.strip_singleton_subgroup => (sub[0].width, 0),
            (None, _) => (sub_max_w, 1),
        };
        emit_with_paddings(source, sub, max_w, suffix, edits);
    }
}

/// Rewrites each member's gap to `suffix_len + (max_w - m.width)`
/// spaces. Members whose gap already carries that width of ASCII
/// spaces emit nothing.
fn emit_with_paddings(
    source: &Source,
    members: &[Member],
    max_w: usize,
    suffix_len: usize,
    edits: &mut Vec<Edit>,
) {
    edits.extend(
        members
            .iter()
            .filter_map(|m| space_padding_edit(source, m.gap, suffix_len + (max_w - m.width))),
    );
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

/// Builds a `Member` for a row whose aligned token sits at `anchor`,
/// with width measured by the display width of `target` plus
/// `extra_width`. Pass `extra_width = 0` when the LHS is exactly
/// `target` (e.g. `x = 1`), and pass a non-zero value when the LHS
/// visually extends past `target` by characters not covered by the
/// slice (e.g. the `+` of `x += 1` widens the LHS by one column
/// without being part of the target range).
fn range_anchored_member(
    source: &Source,
    target: TextRange,
    anchor: TextSize,
    extra_width: usize,
) -> Member {
    Member {
        gap: TextRange::new(target.end(), anchor),
        line_start: source.text().line_start(anchor),
        width: source.slice(target).width() + extra_width,
    }
}

/// Returns the width of the strictly-widest member in `members` when
/// the greedy partition isolates that member as its own sub-group.
fn widest_singleton_anchor_width(
    members: &[Member],
    subs: &[(usize, usize, usize)],
) -> Option<usize> {
    let (widest_idx, widest) = members.iter().enumerate().max_by_key(|(_, m)| m.width)?;
    let w = widest.width;
    let unique = members.iter().filter(|m| m.width == w).nth(1).is_none();
    let isolated = subs
        .iter()
        .any(|&(s, e, _)| s == widest_idx && e == widest_idx + 1);
    (unique && isolated).then_some(w)
}

#[cfg(test)]
mod tests {
    use ruff_text_size::TextSize;

    use super::*;
    use crate::test_support::parse;

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
                gap: TextRange::new(TextSize::new(gap_start), TextSize::new(gap_end)),
                line_start: TextSize::new(line_start),
                width,
            });
        }
        (parse(&text), members)
    }

    /// Builds a `Settings` carrying the test's cap and policy with
    /// `strip_singleton_subgroup` defaulted off.
    fn settings(max_shift: usize, policy: MaxAlignShiftPolicy) -> Settings {
        Settings {
            max_shift,
            policy,
            strip_singleton_subgroup: false,
        }
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
    fn emit_group_skip_emits_nothing_when_spread_exceeds_cap() {
        let (source, members) = rows(&[(1, 1), (2, 1), (15, 1), (3, 1)]);
        let mut edits = Vec::new();

        emit_group(
            &source,
            &members,
            settings(8, MaxAlignShiftPolicy::Skip),
            &mut edits,
        );

        assert!(edits.is_empty(), "skip policy must not emit edits");
    }

    #[test]
    fn emit_group_split_anchors_at_widest_singleton_when_strip_is_set() {
        // Widths span 13 → 4, a 9-wide spread that exceeds max_shift=8.
        // The greedy partition isolates the leading width-13 member as
        // a singleton. With strip on, the singleton anchors the whole
        // group at width 13 + 1, so every member's `:` lands at the
        // same column: 13+1, 4+10, 11+3, 7+7 = 14.
        let (source, members) = rows(&[(13, 1), (4, 1), (11, 1), (7, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // member[0] already carries gap=1 (the target), so no edit emits.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[1], 10),
                fill(&members[2], 3),
                fill(&members[3], 7),
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
    fn emit_group_split_skips_anchor_when_widest_width_ties() {
        // widths 13, 4, 13 yield spread = 9, exceeding max_shift=8.
        // Partition isolates each as a singleton. Both 13s share max
        // width, so no strictly-widest singleton anchor fires, and
        // strip collapses each gap to zero.
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
        emit_with_paddings(&source, &members, members[0].width, 0, &mut edits);

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
        emit_with_paddings(&source, &members, members[0].width, 1, &mut edits);

        assert!(
            edits.is_empty(),
            "gap that already matches the target width must not emit",
        );
    }

    #[test]
    fn keyed_line_adjacent_groups_flushes_trailing_active_run() {
        let source = parse("x = 1\ny = 2\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_merges_same_key_adjacent_stmts() {
        let source = parse("x = 1\ny = 2\nz = 3\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn keyed_line_adjacent_groups_non_qualifier_closes_active_run() {
        let source = parse("x = 1\npass\ny = 2\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_returns_empty_for_empty_body() {
        let source = parse("");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert!(groups.is_empty());
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_blank_line() {
        let source = parse("x = 1\n\ny = 2\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_comment_in_gap() {
        let source = parse("x = 1\n# comment\ny = 2\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_key_change_at_adjacent_boundary() {
        // Two assigns flanking an aug-assign, all line-adjacent. The
        // distinct keys force the run to split even though no whitespace
        // breaks the adjacency, exercising the `keyed`-only invariant.
        let source = parse("x = 1\ny += 2\nz = 3\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            if s.is_assign_stmt() {
                Some(("assign", ()))
            } else if s.is_aug_assign_stmt() {
                Some(("aug", ()))
            } else {
                None
            }
        });

        assert_eq!(
            groups.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![1, 1, 1],
        );
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_multiline_prior_stmt() {
        let source = parse("x = {\n    'a': 1,\n}\ny = 2\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_yields_singleton_for_lone_qualifier() {
        let source = parse("x = 1\n");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
    }

    #[test]
    fn line_anchored_member_collapses_gap_at_line_start() {
        let source = parse("xy\n");
        let member = line_anchored_member(&source, TextSize::new(0));

        // anchor sits at line start, with empty gap and zero width.
        assert_eq!(member.gap.start(), member.gap.end());
        assert_eq!(member.width, 0);
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

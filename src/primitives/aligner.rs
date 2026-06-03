//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. Each rule wraps an
//! `AlignWalker` whose `emit_group` method drives the math. Per-rule
//! knobs travel through `Settings`. Aligned rows always carry a
//! one-space buffer between content and the aligned token.

use std::num::NonZeroUsize;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyParameterRef, Parameters, Stmt,
    token::{Token, TokenKind},
};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::{AlignImportsConfig, AlignmentConfig, MaxAlignShiftPolicy},
    rule::RuleId,
    source::Source,
};

/// Bundles the `groups` accumulator, `settings`, the owning `rule`, and
/// borrowed `source` shared by every alignment-rule visitor. Each entry
/// in `groups` is one fix the pipeline maps to a single diagnostic. The
/// `rule` id powers the skip-directive check that holds a row out of
/// its group.
pub(crate) struct AlignWalker<'a> {
    pub groups: Vec<Vec<Edit>>,
    pub rule: RuleId,
    pub settings: Settings,
    pub source: &'a Source,
}

impl<'a> AlignWalker<'a> {
    /// Builds a walker with an empty `groups` accumulator.
    pub(crate) fn new(source: &'a Source, settings: Settings, rule: RuleId) -> Self {
        Self {
            groups: Vec::new(),
            rule,
            settings,
            source,
        }
    }

    /// Aligns `members` as one fix group, recording it when the pass
    /// rewrites at least one gap.
    pub(crate) fn emit_group(&mut self, members: &[Member]) {
        let edits = self.group_edits(members);
        self.push_group(edits);
    }

    /// Drops the held rows from `members`, then emits the survivors as
    /// one group when they still form an alignment candidate.
    pub(crate) fn emit_unheld(&mut self, members: impl IntoIterator<Item = Member>) {
        let kept: Vec<Member> = members
            .into_iter()
            .filter(|m| !self.is_held(m.line_start))
            .collect();
        if is_alignment_candidate(&kept) {
            self.emit_group(&kept);
        }
    }

    /// Computes the alignment edits for `members` without recording
    /// them, leaving the caller to fold in further edits before
    /// committing the group through [`Self::push_group`].
    pub(crate) fn group_edits(&self, members: &[Member]) -> Vec<Edit> {
        let mut edits = Vec::new();
        emit_group(self.source, members, self.settings, &mut edits);
        edits
    }

    /// Returns `true` when `anchor`'s source line is skip-suppressed for
    /// this rule, so the row drops out of its alignment group as a
    /// transparent hole that neighbors still align around.
    pub(crate) fn is_held(&self, anchor: TextSize) -> bool {
        is_held(self.source, self.rule, anchor)
    }

    /// Records `edits` as one fix group, dropping an empty group so a
    /// no-op pass emits no diagnostic.
    pub(crate) fn push_group(&mut self, edits: Vec<Edit>) {
        if !edits.is_empty() {
            self.groups.push(edits);
        }
    }
}

/// One row in an alignment group.
///
/// `width` is the display-column width of the row's left-hand-side
/// region, from the start of the member to the start of the gap. `gap`
/// is the whitespace range ending immediately before the aligned
/// token that the rule will rewrite. `line_start` is the offset of
/// the start of the source line containing the gap. `op_width` is the
/// display width of the aligned operator itself, used to right-align
/// variable-width operators within a group. Rules with fixed-width
/// operators leave `op_width` at zero.
#[derive(Clone, Copy)]
pub(crate) struct Member {
    pub gap: TextRange,
    pub line_start: TextSize,
    pub op_width: usize,
    pub width: usize,
}

impl Member {
    /// Returns a copy of `self` with `op_width` set to the operator's
    /// display width, opting the member into right-alignment math.
    pub(crate) fn with_op_width(mut self, op_width: usize) -> Self {
        self.op_width = op_width;
        self
    }
}

/// Emission knobs shared by every alignment rule.
///
/// `strip_singleton_subgroup` collapses size-one sub-groups in
/// `emit_split` to a zero-width gap.
#[derive(Clone, Copy)]
pub(crate) struct Settings {
    max_shift: usize,
    policy: MaxAlignShiftPolicy,
    strip_singleton_subgroup: bool,
}

impl Settings {
    /// Builds the alignment settings carried by an alignment rule, with
    /// `strip_singleton_subgroup` off until a rule opts in.
    fn aligned(max_shift: NonZeroUsize, policy: MaxAlignShiftPolicy) -> Self {
        Self {
            max_shift: max_shift.get(),
            policy,
            strip_singleton_subgroup: false,
        }
    }

    /// Returns a copy of `self` with `strip_singleton_subgroup` enabled.
    pub(crate) fn with_singleton_subgroup_strip(mut self) -> Self {
        self.strip_singleton_subgroup = true;
        self
    }
}

impl From<&AlignImportsConfig> for Settings {
    fn from(c: &AlignImportsConfig) -> Self {
        Self::aligned(c.max_shift, c.max_shift_policy)
    }
}

impl From<&AlignmentConfig> for Settings {
    fn from(c: &AlignmentConfig) -> Self {
        Self::aligned(c.max_shift, c.max_shift_policy)
    }
}

/// Moves the in-progress run into `groups` when it holds at least one
/// member, leaving `current` empty for the next run.
pub(crate) fn flush_run<M>(groups: &mut Vec<Vec<M>>, current: &mut Vec<M>) {
    if !current.is_empty() {
        groups.push(std::mem::take(current));
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

/// Returns `true` when the line containing `anchor` carries a skip
/// directive for `rule`: a bare `# prose: skip` / `# fmt: skip` span, a
/// `# fmt: off` region, or `# prose: skip[rule]`. A held row stays out
/// of the column math and emits no edit, so its neighbors align around
/// it. Short-circuits when the source carries no format suppression.
pub(crate) fn is_held(source: &Source, rule: RuleId, anchor: TextSize) -> bool {
    let suppression = source.suppression_map();
    if !suppression.has_format_suppression() && !suppression.has_skip_suppression() {
        return false;
    }
    suppression.intersects(source.text().full_line_range(anchor))
        || suppression.is_format_suppressed_at(source.line_index(anchor), rule)
}

/// Generalization of [`line_adjacent_groups`] for rules that admit
/// more than one member shape. The qualifier returns `Option<(K, M)>`
/// where `K` tags the shape, and a run extends only while the next
/// member shares the active key, sits line-adjacent to the prior
/// statement, and the prior statement itself fits on one source line.
/// A key change at an otherwise-adjacent boundary closes the active
/// run and starts a fresh one without losing the boundary statement.
/// A statement [held](is_held) for `rule` is transparent: it joins no
/// group and does not close the run, leaving neighbors on either side
/// to align as one block. Adjacency across a held statement relaxes to
/// a consecutive-line check, so the held row's own trailing skip
/// comment does not break the run while a standalone comment or blank
/// line between rows still does. Walks `body` exactly once.
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
    let mut active: Option<(K, TextRange, bool)> = None;
    for stmt in body {
        let Some((key, member)) = qualify(stmt) else {
            flush_run(&mut groups, &mut current);
            active = None;
            continue;
        };
        if is_held(source, rule, stmt.start()) {
            if let Some((_, prev, prev_held)) = active.as_mut() {
                *prev = stmt.range();
                *prev_held = true;
            }
            continue;
        }
        let extends = active
            .as_ref()
            .is_some_and(|(active_key, prev, prev_held)| {
                active_key == &key
                    && !source.contains_line_break(prev)
                    && run_continues(source, prev.end(), *prev_held, stmt.start())
            });
        if !extends {
            flush_run(&mut groups, &mut current);
        }
        current.push(member);
        active = Some((key, stmt.range(), false));
    }
    flush_run(&mut groups, &mut current);
    groups
}

/// Walks `body`, qualifying each statement through `qualify` and
/// grouping the qualified members into runs where every consecutive
/// pair sits on adjacent source lines. A multi-line prior statement,
/// a non-qualifying statement, a comment in the inter-statement gap,
/// or a blank line breaks the current run. A statement held for `rule`
/// is transparent per [`keyed_line_adjacent_groups`]. Empty groups
/// (statements that fail qualification with no qualified neighbors) are
/// skipped. Thin wrapper over [`keyed_line_adjacent_groups`] for rules
/// whose qualifier produces only one form, so every member shares an
/// implicit `()` key.
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
        op_width: 0,
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
    emit_with_paddings(source, kept, max_w, max_op_width(kept), 1, edits);
}

/// Aligns the members of a group, dispatching through `settings.policy`
/// when the widest padding exceeds `settings.max_shift`. A singleton
/// group collapses its gap to one space, or to zero when
/// `settings.strip_singleton_subgroup` is set, matching the
/// singleton-from-split treatment in `emit_split`.
fn emit_group(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
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
        op_width: 0,
        width: source.slice(target).width() + extra_width,
    }
}

/// Returns whether a run continues from a row ending at `prev_end` to
/// the next row starting at `next_start`. A non-held predecessor uses
/// the standard inter-statement adjacency. A [held](is_held)
/// predecessor relaxes to a consecutive-line check, so the held row's
/// own trailing skip comment does not break the run while a standalone
/// comment or blank line between rows still does.
fn run_continues(
    source: &Source,
    prev_end: TextSize,
    prev_held: bool,
    next_start: TextSize,
) -> bool {
    if prev_held {
        source.consecutive_lines(prev_end, next_start)
    } else {
        source.is_line_adjacent(TextRange::new(prev_end, next_start))
    }
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
    fn keyed_line_adjacent_groups_breaks_on_blank_line_after_held() {
        let source = parse("x = 1\ny = 2  # prose: skip[align-equals]\n\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_breaks_on_standalone_comment_after_held() {
        let source = parse("x = 1\ny = 2  # prose: skip[align-equals]\n# note\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        // A standalone comment after the held line is not consecutive,
        // so the relaxed adjacency still breaks the run.
        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_flushes_trailing_active_run() {
        let source = parse("x = 1\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_holds_member_with_extra_comment_on_its_line() {
        let source = parse("x = 1\ny = 2  # note  # prose: skip[align-equals]\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        // The held line's extra trailing comment rides along with it,
        // so x and z still bridge across it into one run.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_holds_skip_suppressed_member_and_bridges_run() {
        let source = parse("x = 1\ny = 2  # prose: skip[align-equals]\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        // y is held, so it joins no group, yet x and z bridge across it
        // into one run.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_merges_same_key_adjacent_stmts() {
        let source = parse("x = 1\ny = 2\nz = 3\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn keyed_line_adjacent_groups_non_qualifier_closes_active_run() {
        let source = parse("x = 1\npass\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_returns_empty_for_empty_body() {
        let source = parse("");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert!(groups.is_empty());
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_blank_line() {
        let source = parse("x = 1\n\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_comment_in_gap() {
        let source = parse("x = 1\n# comment\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
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

    #[test]
    fn keyed_line_adjacent_groups_splits_on_multiline_prior_stmt() {
        let source = parse("x = {\n    'a': 1,\n}\ny = 2\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_yields_singleton_for_lone_qualifier() {
        let source = parse("x = 1\n");
        let groups = keyed_line_adjacent_groups(
            &source,
            &source.ast().body,
            RuleId::from("align-equals"),
            |s| s.as_assign_stmt().map(|_| ((), ())),
        );

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

//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. `emit_group` is the
//! entry point. Per-rule knobs travel through `Settings`, where every
//! alignment rule uses `suffix_len = 1`, leaving a one-space buffer
//! before the aligned token.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::Stmt;
use ruff_python_trivia::{lines_before, PythonWhitespace};
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
/// the start of the source line containing the gap, captured at
/// construction time so [`distinct_lines`] can compare line identity
/// without re-scanning the source.
#[derive(Clone, Copy, Debug)]
pub struct Member {
    pub gap: TextRange,
    pub line_start: TextSize,
    pub width: usize,
}

/// Emission knobs shared by every alignment rule.
///
/// `suffix_len` is the gap between content and aligned token when
/// alignment fires (currently `1` for every rule).
/// `strip_singleton_subgroup` overrides that to `0` for size-one
/// sub-groups in `emit_split`, used by the `:`-anchored rules.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub max_shift: usize,
    pub policy: MaxAlignShiftPolicy,
    pub strip_singleton_subgroup: bool,
    pub suffix_len: usize,
}

impl Settings {
    /// Returns a copy of `self` with `strip_singleton_subgroup` enabled.
    pub fn with_singleton_subgroup_strip(mut self) -> Self {
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
            suffix_len: 1,
        }
    }
}

/// Aligns the members of a group, dispatching through `settings.policy`
/// when the widest padding exceeds `settings.max_shift`. A `Split`
/// sub-group of size one collapses its gap to `settings.suffix_len`
/// spaces, or to zero when `settings.strip_singleton_subgroup` is set.
pub fn emit_group(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
    let Some(first) = members.first() else {
        return;
    };
    let (min_w, max_w) = members
        .iter()
        .fold((first.width, first.width), |(mn, mx), m| {
            (mn.min(m.width), mx.max(m.width))
        });
    if max_w - min_w <= settings.max_shift {
        emit_with_paddings(source, members, max_w, settings.suffix_len, edits);
        return;
    }
    match settings.policy {
        MaxAlignShiftPolicy::Drop => emit_drop(source, members, settings, edits),
        MaxAlignShiftPolicy::Skip => {}
        MaxAlignShiftPolicy::Split => emit_split(source, members, settings, edits),
    }
}

/// Rewrites each member's gap to `suffix_len + (max_w - m.width)`
/// spaces, skipping members whose gap already carries that exact
/// width of ASCII spaces. Emits `Edit::deletion` when the target
/// width is zero and the current gap is non-empty, because
/// `Edit::range_replacement` rejects empty content.
fn emit_with_paddings(
    source: &Source,
    members: &[Member],
    max_w: usize,
    suffix_len: usize,
    edits: &mut Vec<Edit>,
) {
    edits.extend(members.iter().filter_map(|m| {
        let target_len = suffix_len + (max_w - m.width);
        let gap_text = source.slice(m.gap);
        let already_correct = gap_text.len() == target_len && gap_text.bytes().all(|b| b == b' ');
        if already_correct {
            return None;
        }
        if target_len == 0 {
            return Some(Edit::range_deletion(m.gap));
        }
        Some(Edit::range_replacement(" ".repeat(target_len), m.gap))
    }));
}

/// Returns `true` when the gap between two AST nodes carries exactly
/// one newline and no comment, meaning the surrounding nodes sit on
/// directly adjacent source lines. Backs `line_adjacent_groups`, which
/// is the public interface every alignment rule consumes.
fn is_line_adjacent(source: &Source, gap: TextRange) -> bool {
    !source.slice(gap).contains('#') && lines_before(gap.end(), source.text()) == 1
}

/// Builds a `Member` for a row whose aligned token sits at `anchor`.
/// Width is the display width of the line's content from the first
/// non-whitespace character to the last non-whitespace character
/// before the gap, leaving the gap free for the rule to rewrite.
/// Shared by every rule that aligns at a token's line position.
pub fn line_anchored_member(source: &Source, anchor: TextSize) -> Member {
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

/// Builds a `Member` for a row whose aligned token sits at `anchor`,
/// with width measured by the display width of `target` plus
/// `extra_width`. Pass `extra_width = 0` when the LHS is exactly
/// `target` (e.g. `x = 1`), and pass a non-zero value when the LHS
/// visually extends past `target` by characters not covered by the
/// slice (e.g. the `+` of `x += 1` widens the LHS by one column
/// without being part of the target range).
pub fn range_anchored_member(
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

/// Returns `true` when every member's aligned token sits on a
/// distinct source line. Two `:`-anchored members on the same line
/// have no column to align to, so rules that align across lines key
/// off this predicate to decide whether alignment applies.
pub fn distinct_lines(members: &[Member]) -> bool {
    members
        .windows(2)
        .all(|w| w[0].line_start != w[1].line_start)
}

/// Builds a `Member` whose anchor is the first token of `kind` within
/// `search`. Convenience for the dominant rule-side composition: find
/// a keyword or operator token in a tight range, then build a
/// line-anchored member from its start. Returns `None` when the search
/// turns up nothing.
pub fn line_anchored_member_at_kind(
    source: &Source,
    search: TextRange,
    kind: TokenKind,
) -> Option<Member> {
    let anchor = source.first_token_offset_in_range(search, |t| t.kind() == kind)?;
    Some(line_anchored_member(source, anchor))
}

/// Builds a `Member` whose anchor is the first token in `search`
/// satisfying `predicate`, with width measured by `target` plus
/// `extra_width`. Returns `None` if no token matches, or if the span
/// from `target.start()` to the anchor crosses a newline (continuation
/// imports, line-broken assignments). Use this when alignment must
/// stay confined to a single source line.
pub fn range_anchored_member_single_line<F>(
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
    if source
        .text()
        .contains_line_break(TextRange::new(target.start(), anchor))
    {
        return None;
    }
    Some(range_anchored_member(source, target, anchor, extra_width))
}

/// Walks `body`, qualifying each statement through `qualify` and
/// grouping the qualified members into runs where every consecutive
/// pair sits on adjacent source lines. A non-qualifying statement, a
/// comment in the inter-statement gap, or a blank line breaks the
/// current run. Empty groups (statements that fail qualification with
/// no qualified neighbors) are skipped. Thin wrapper over
/// [`keyed_line_adjacent_groups`] for rules whose qualifier produces
/// only one form, so every member shares an implicit `()` key.
pub fn line_adjacent_groups<'a, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    mut qualify: F,
) -> Vec<Vec<M>>
where
    F: FnMut(&'a Stmt) -> Option<M>,
{
    keyed_line_adjacent_groups(source, body, move |stmt| qualify(stmt).map(|m| ((), m)))
}

/// Generalization of [`line_adjacent_groups`] for rules that admit
/// more than one member shape. The qualifier returns `Option<(K, M)>`
/// where `K` tags the shape, and a run extends only while the next
/// member shares both the active key and line-adjacency. A key change
/// at an otherwise-adjacent boundary closes the active run and starts
/// a fresh one without losing the boundary statement, which keeps
/// `align_imports` from mixing `from`-import members with
/// `import M as A` members in a single group. Walks `body` exactly
/// once and calls `is_line_adjacent` at most once per qualifying
/// statement.
pub fn keyed_line_adjacent_groups<'a, K, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    mut qualify: F,
) -> Vec<Vec<M>>
where
    K: Eq,
    F: FnMut(&'a Stmt) -> Option<(K, M)>,
{
    let mut groups: Vec<Vec<M>> = Vec::new();
    let mut active: Option<(K, TextSize)> = None;
    for stmt in body {
        let Some((key, member)) = qualify(stmt) else {
            active = None;
            continue;
        };
        let extends = active.as_ref().is_some_and(|(active_key, last_end)| {
            active_key == &key && is_line_adjacent(source, TextRange::new(*last_end, stmt.start()))
        });
        if extends {
            groups
                .last_mut()
                .expect("active implies groups non-empty")
                .push(member);
        } else {
            groups.push(vec![member]);
        }
        active = Some((key, stmt.end()));
    }
    groups
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
    emit_with_paddings(source, kept, max_w, settings.suffix_len, edits);
}

/// Greedy partitioning: extends the current sub-group while its
/// widest padding stays under the cap, then starts a new sub-group.
/// Each contiguous sub-group aligns independently. A singleton
/// sub-group collapses its gap to `suffix_len` by default, or to
/// zero when `settings.strip_singleton_subgroup` is set.
fn emit_split(source: &Source, members: &[Member], settings: Settings, edits: &mut Vec<Edit>) {
    let mut cursor = 0;
    while cursor < members.len() {
        let mut min_w = members[cursor].width;
        let mut max_w = min_w;
        let mut end = cursor + 1;
        while end < members.len() {
            let w = members[end].width;
            let new_min = min_w.min(w);
            let new_max = max_w.max(w);
            if new_max - new_min > settings.max_shift {
                break;
            }
            min_w = new_min;
            max_w = new_max;
            end += 1;
        }
        let sub = &members[cursor..end];
        let suffix = if sub.len() == 1 && settings.strip_singleton_subgroup {
            0
        } else {
            settings.suffix_len
        };
        emit_with_paddings(source, sub, max_w, suffix, edits);
        cursor = end;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ruff_text_size::TextSize;

    use super::*;

    /// Builds the expected `(start, end, content)` tuple for an edit
    /// that rewrites a member's gap to `n` spaces.
    fn fill(member: &Member, n: usize) -> (u32, u32, String) {
        (
            member.gap.start().to_u32(),
            member.gap.end().to_u32(),
            " ".repeat(n),
        )
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

    /// Builds a multi-line Python source where each row is
    /// `x...x{spaces}= 0\n`, returns the source plus one `Member` per
    /// row pointing at that row's pre-`=` whitespace. The `gap_chars`
    /// value seeds the existing pre-`=` whitespace so tests can probe
    /// the "already correct" branch in `emit_with_paddings`.
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
        (
            Source::from_str(&text).expect("test source parses"),
            members,
        )
    }

    /// Builds a `Settings` carrying the test's cap and policy. All
    /// inline tests use `suffix_len = 1`, matching every rule in
    /// production. `strip_singleton_subgroup` defaults off; the one
    /// test that exercises the strip path enables it explicitly.
    fn settings(max_shift: usize, policy: MaxAlignShiftPolicy) -> Settings {
        Settings {
            max_shift,
            policy,
            strip_singleton_subgroup: false,
            suffix_len: 1,
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
        let source = Source::from_str("x = 0\n").expect("parses");
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
    fn emit_group_split_strips_singleton_subgroup_when_flag_is_set() {
        let (source, members) = rows(&[(1, 1), (2, 1), (15, 1), (3, 1), (4, 1)]);
        let mut edits = Vec::new();

        let settings = settings(8, MaxAlignShiftPolicy::Split).with_singleton_subgroup_strip();
        emit_group(&source, &members, settings, &mut edits);

        // [15] singleton sub-group collapses to 0 with strip on.
        assert_eq!(
            sorted_summaries(&edits),
            vec![
                fill(&members[0], 2),
                delete(&members[2]),
                fill(&members[3], 2)
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
        // aligns at max=4. After alignment: targets are 2/1/1/2/1
        // spaces. members 1, 2, 4 already have 1 space.
        assert_eq!(
            sorted_summaries(&edits),
            vec![fill(&members[0], 2), fill(&members[3], 2)],
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
        let source = Source::from_str("x = 1\ny = 2\n").expect("parses");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn keyed_line_adjacent_groups_merges_same_key_adjacent_stmts() {
        let source = Source::from_str("x = 1\ny = 2\nz = 3\n").expect("parses");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn keyed_line_adjacent_groups_non_qualifier_closes_active_run() {
        let source = Source::from_str("x = 1\npass\ny = 2\n").expect("parses");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_returns_empty_for_empty_body() {
        let source = Source::from_str("").expect("parses");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert!(groups.is_empty());
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_blank_line() {
        let source = Source::from_str("x = 1\n\ny = 2\n").expect("parses");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
    }

    #[test]
    fn keyed_line_adjacent_groups_splits_on_comment_in_gap() {
        let source = Source::from_str("x = 1\n# comment\ny = 2\n").expect("parses");
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
        let source = Source::from_str("x = 1\ny += 2\nz = 3\n").expect("parses");
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
    fn keyed_line_adjacent_groups_yields_singleton_for_lone_qualifier() {
        let source = Source::from_str("x = 1\n").expect("parses");
        let groups = keyed_line_adjacent_groups(&source, &source.ast().body, |s| {
            s.as_assign_stmt().map(|_| ((), ()))
        });

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
    }

    #[test]
    fn line_anchored_member_collapses_gap_at_line_start() {
        let source = Source::from_str("xy\n").expect("parses");
        let member = line_anchored_member(&source, TextSize::new(0));

        // anchor sits at line start: empty gap, zero width.
        assert_eq!(member.gap.start(), member.gap.end());
        assert_eq!(member.width, 0);
    }
}

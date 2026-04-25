//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. `emit_group` is the
//! entry point. Per-rule knobs travel through `Settings`; both
//! `align_colons` and `align_equals` use `suffix_len = 1`, leaving a
//! one-space buffer before the aligned token.

use ruff_diagnostics::Edit;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};

use crate::config::MaxAlignShiftPolicy;
use crate::source::Source;

/// One row in an alignment group.
///
/// `width` is the display-column width of the row's left-hand-side
/// region, from the start of the member to the start of the gap. `gap`
/// is the whitespace range ending immediately before the aligned
/// token that the rule will rewrite.
#[derive(Clone, Copy, Debug)]
pub struct Member {
    pub gap: TextRange,
    pub width: usize,
}

/// Emission knobs shared by every alignment rule.
///
/// `suffix_len` is the fixed gap between the rightmost content character
/// and the aligned token (currently `1` for both `align_colons` and
/// `align_equals`, leaving a one-space buffer before the token).
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub max_shift: usize,
    pub policy: MaxAlignShiftPolicy,
    pub suffix_len: usize,
}

/// Aligns the members of a group, dispatching through `settings.policy`
/// when the widest padding exceeds `settings.max_shift`. A `Split`
/// sub-group of size one collapses its gap to `settings.suffix_len`
/// spaces.
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
        let paddings: Vec<usize> = members.iter().map(|m| max_w - m.width).collect();
        emit_with_paddings(source, members, &paddings, settings.suffix_len, edits);
        return;
    }
    match settings.policy {
        MaxAlignShiftPolicy::Drop => emit_drop(source, members, settings, edits),
        MaxAlignShiftPolicy::Skip => {}
        MaxAlignShiftPolicy::Split => emit_split(source, members, settings, edits),
    }
}

/// Rewrites each member's gap to `suffix_len + paddings[i]` spaces,
/// skipping members whose gap already carries that exact width of
/// ASCII spaces. Emits `Edit::deletion` when the target width is
/// zero and the current gap is non-empty, since `Edit::range_replacement`
/// rejects empty content. Parallel-slice API: `members.len() == paddings.len()`.
fn emit_with_paddings(
    source: &Source,
    members: &[Member],
    paddings: &[usize],
    suffix_len: usize,
    edits: &mut Vec<Edit>,
) {
    edits.extend(members.iter().zip(paddings).filter_map(|(m, &p)| {
        let target_len = suffix_len + p;
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

/// Returns `true` when the token gap between two AST nodes carries
/// exactly one newline and no comment, meaning the surrounding nodes
/// sit on directly adjacent source lines. Backs `line_adjacent_groups`,
/// which is the public interface every alignment rule consumes.
fn is_line_adjacent(source: &Source, gap: TextRange) -> bool {
    source
        .tokens()
        .in_range(gap)
        .iter()
        .try_fold(0usize, |n, t| match t.kind() {
            k if k.is_comment() => None,
            k if k.is_any_newline() => Some(n + 1),
            _ => Some(n),
        })
        == Some(1)
}

/// Walks `body`, qualifying each statement through `qualify` and
/// grouping the qualified members into runs where every consecutive
/// pair sits on adjacent source lines. A non-qualifying statement, a
/// comment in the inter-statement gap, or a blank line breaks the
/// current run. Empty groups (statements that fail qualification with
/// no qualified neighbors) are skipped.
pub fn line_adjacent_groups<'a, M, F>(
    source: &'a Source,
    body: &'a [Stmt],
    mut qualify: F,
) -> Vec<Vec<M>>
where
    F: FnMut(&'a Stmt) -> Option<M>,
{
    let mut groups = Vec::new();
    let mut iter = body.iter().peekable();
    while let Some(stmt) = iter.next() {
        let Some(first) = qualify(stmt) else {
            continue;
        };
        let mut cursor_end = stmt.range().end();
        let mut members = vec![first];
        while let Some(&next_stmt) = iter.peek() {
            let gap = TextRange::new(cursor_end, next_stmt.range().start());
            if !is_line_adjacent(source, gap) {
                break;
            }
            let Some(next) = qualify(next_stmt) else {
                break;
            };
            cursor_end = next_stmt.range().end();
            members.push(next);
            iter.next();
        }
        groups.push(members);
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
    let paddings: Vec<usize> = kept.iter().map(|m| max_w - m.width).collect();
    emit_with_paddings(source, kept, &paddings, settings.suffix_len, edits);
}

/// Greedy partitioning: extends the current sub-group while its
/// widest padding stays under the cap, then starts a new sub-group.
/// Each contiguous sub-group aligns independently. A singleton
/// sub-group collapses its gap to `suffix_len` spaces.
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
        let paddings: Vec<usize> = sub.iter().map(|m| max_w - m.width).collect();
        emit_with_paddings(source, sub, &paddings, settings.suffix_len, edits);
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

    /// Builds a multi-line Python source where each row is
    /// `x...x{spaces}= 0\n`, returns the source plus one `Member` per
    /// row pointing at that row's pre-`=` whitespace. The `gap_chars`
    /// value seeds the existing pre-`=` whitespace so tests can probe
    /// the "already correct" branch in `emit_with_paddings`.
    fn rows(specs: &[(usize, usize)]) -> (Source, Vec<Member>) {
        let mut text = String::new();
        let mut members = Vec::new();
        for &(width, gap_chars) in specs {
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
                width,
            });
        }
        (
            Source::from_str(&text).expect("test source parses"),
            members,
        )
    }

    fn sorted_summaries(edits: &[Edit]) -> Vec<(u32, u32, String)> {
        let mut out: Vec<_> = edits.iter().map(summary).collect();
        out.sort();
        out
    }

    /// Builds a `Settings` carrying the test's cap and policy. All
    /// inline tests use `suffix_len = 1`, matching both rules in production.
    fn settings(max_shift: usize, policy: MaxAlignShiftPolicy) -> Settings {
        Settings {
            max_shift,
            policy,
            suffix_len: 1,
        }
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

        emit_with_paddings(&source, &members, &[0], 0, &mut edits);

        // target_len = 0 + 0 → must emit deletion (range_replacement
        // rejects empty content). Deletion has no content, so the
        // summary's content slot is the empty string.
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

        emit_with_paddings(&source, &members, &[0], 1, &mut edits);

        assert!(
            edits.is_empty(),
            "gap that already matches the target width must not emit",
        );
    }
}

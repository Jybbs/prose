//! Unified-diff excerpts a sweep report shows beside a defect.

use std::ops::RangeBounds;

use itertools::Itertools;
use similar::{DiffOp, DiffTag, TextDiff};

use super::{more, with_rest};

/// How many lines of a diff an excerpt keeps before it reports the
/// remainder as a count.
pub(crate) const EXCERPT: usize = 16;

/// Renders the first hunk of the unified diff from `before` to `after`
/// that changes one of the zero-based `rows` of `before`, headed `from`
/// and `to`. The excerpt stops at [`EXCERPT`] lines and reports the rest
/// of the diff as a hunk count and a line count.
pub(crate) fn excerpt(
    from: &str,
    to: &str,
    before: &str,
    after: &str,
    rows: impl RangeBounds<usize>,
) -> String {
    let diff = TextDiff::from_lines(before, after);
    let hunks = diff.unified_diff().iter_hunks().collect_vec();
    let Some(first) = hunks
        .iter()
        .find(|hunk| hunk.ops().iter().any(|op| changes(op, &rows)))
    else {
        return String::new();
    };
    let rest = hunks.len() - 1;
    let first = first.to_string();
    let lines: Vec<&str> = first.lines().collect();
    let shown = format!(
        "--- {from}\n+++ {to}\n{}",
        lines.iter().take(EXCERPT).format("\n")
    );
    let head = match lines.len().saturating_sub(EXCERPT) {
        0 if rest == 0 => return shown,
        0 => "...".to_owned(),
        cut => format!("... {}", more(cut, "line")),
    };
    format!("{shown}\n{}", with_rest(&head, rest, "hunk"))
}

/// The whole unified diff from `expected` to `actual`, headed by those
/// two words, which a divergence check prints in full rather than
/// excerpting.
pub(crate) fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}

/// Reports whether `op` deletes, replaces, or inserts at one of `rows`, an
/// insertion counting at the row it lands ahead of.
fn changes(op: &DiffOp, rows: &impl RangeBounds<usize>) -> bool {
    let old = op.old_range();
    op.tag() != DiffTag::Equal
        && (old.start..old.end.max(old.start + 1)).any(|row| rows.contains(&row))
}

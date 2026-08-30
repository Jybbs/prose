//! Unified-diff excerpts a sweep report shows beside a defect.

use itertools::Itertools;
use similar::TextDiff;

/// How many lines of a diff an excerpt keeps before it reports the
/// remainder as a count.
pub(crate) const EXCERPT: usize = 16;

/// The first hunk of the unified diff from `before` to `after`, headed
/// `from` and `to`, capped at [`EXCERPT`] lines with the remainder of
/// the diff reported as a hunk count and a line count.
pub(crate) fn excerpt(from: &str, to: &str, before: &str, after: &str) -> String {
    let diff = TextDiff::from_lines(before, after);
    let mut hunks = diff.unified_diff().iter_hunks();
    let Some(first) = hunks.next() else {
        return String::new();
    };
    let rest = hunks.count();
    let first = first.to_string();
    let lines: Vec<&str> = first.lines().collect();
    let shown = format!(
        "--- {from}\n+++ {to}\n{}",
        lines.iter().take(EXCERPT).format("\n")
    );
    let more_lines = lines.len().saturating_sub(EXCERPT);
    match (more_lines, rest) {
        (0, 0) => shown,
        (0, hunks) => format!("{shown}\n... and {hunks} more hunks"),
        (lines, 0) => format!("{shown}\n... {lines} more lines"),
        (lines, hunks) => format!("{shown}\n... {lines} more lines and {hunks} more hunks"),
    }
}

pub(crate) fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}

//! Unified-diff excerpts a sweep report shows beside a defect.

use std::ops::RangeBounds;

use itertools::Itertools;
use similar::{DiffOp, DiffTag, TextDiff};

use super::{more, with_rest};

/// How many lines of a diff an excerpt keeps before it reports the
/// remainder as a count.
const EXCERPT: usize = 16;

/// Renders the first hunk of the unified diff from `before` to `after`
/// that changes one of the zero-based `rows` of `before`, headed `from`
/// and `to`. The excerpt stops at [`EXCERPT`] lines and reports the rest
/// of the diff as a hunk count and a line count.
pub fn excerpt(
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
pub fn unified_diff(expected: &str, actual: &str) -> String {
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

#[cfg(test)]
mod tests {
    use std::ops::RangeBounds;

    use rstest::rstest;

    use super::*;

    /// `count` lines reading `word 1` through `word count`.
    fn numbered(word: &str, count: usize) -> String {
        (1..=count).map(|n| format!("{word} {n}\n")).collect()
    }

    #[test]
    fn excerpt_counts_an_insertion_at_the_row_it_lands_ahead_of() {
        let before = numbered("line", 20);
        let after = before.replace("line 10\n", "line 10\ninserted\n");

        assert!(excerpt("before", "after", &before, &after, 10..=10).contains("+inserted"));
        assert!(excerpt("before", "after", &before, &after, 9..=9).is_empty());
    }

    #[test]
    fn excerpt_counts_both_the_lines_and_the_hunks_past_its_cap() {
        let before = numbered("line", 60);
        let after: String = (1..=60)
            .map(|n| {
                if n <= 30 || n == 55 {
                    format!("row {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect();

        let shown = excerpt("before", "after", &before, &after, ..);

        assert!(shown.ends_with(" more lines and 1 more hunk"), "{shown}");
    }

    #[test]
    fn excerpt_counts_one_line_past_its_cap_in_the_singular() {
        let before = numbered("line", 20);
        let after: String = (1..=20)
            .map(|n| {
                if (6..=10).contains(&n) {
                    format!("row {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect();

        let shown = excerpt("before", "after", &before, &after, ..);

        assert!(shown.ends_with("\n... 1 more line"), "{shown}");
    }

    #[test]
    fn excerpt_counts_the_lines_past_its_cap() {
        let before = numbered("line", 40);
        let after = numbered("row", 40);

        let shown = excerpt("before", "after", &before, &after, ..);

        assert_eq!(shown.lines().count(), 2 + EXCERPT + 1, "{shown}");
        assert!(shown.ends_with(" more lines"), "{shown}");
    }

    #[test]
    fn excerpt_ends_on_the_hunk_when_nothing_is_cut() {
        let before = numbered("line", 10);
        let after = before.replace("line 2\n", "line two\n");

        let shown = excerpt("before", "after", &before, &after, ..);

        assert!(!shown.contains("..."), "{shown}");
        assert_eq!(shown.lines().count(), 2 + 7, "{shown}");
    }

    #[test]
    fn excerpt_is_empty_when_no_hunk_changes_the_rows_it_names() {
        let before = numbered("line", 60);
        let after = before.replace("line 2\n", "line two\n");

        assert!(excerpt("before", "after", &before, &after, 30..40).is_empty());
    }

    #[test]
    fn excerpt_is_empty_when_the_texts_match() {
        assert!(excerpt("before", "after", "x = 1\n", "x = 1\n", ..).is_empty());
    }

    #[test]
    fn excerpt_shows_the_first_hunk_and_counts_the_rest() {
        let before = numbered("line", 60);
        let after = before
            .replace("line 2\n", "line two\n")
            .replace("line 50\n", "line fifty\n");

        let shown = excerpt("first pass", "second pass", &before, &after, ..);

        assert!(
            shown.starts_with("--- first pass\n+++ second pass\n@@"),
            "{shown}"
        );
        assert!(shown.contains("-line 2\n+line two\n"), "{shown}");
        assert!(shown.ends_with("... and 1 more hunk"), "{shown}");
    }

    #[rstest]
    #[case(40..60, "-line 50\n+line fifty\n")]
    #[case(49..=49, "-line 50\n+line fifty\n")]
    #[case(0..10, "-line 2\n+line two\n")]
    fn excerpt_shows_the_first_hunk_changing_the_rows_it_names(
        #[case] rows: impl RangeBounds<usize>,
        #[case] hunk: &str,
    ) {
        let before = numbered("line", 60);
        let after = before
            .replace("line 2\n", "line two\n")
            .replace("line 50\n", "line fifty\n");

        let shown = excerpt("before", "after", &before, &after, rows);

        assert!(shown.contains(hunk), "{shown}");
        assert!(shown.ends_with("... and 1 more hunk"), "{shown}");
    }
}

//! Clears the full lines of dropped statements together with the blank
//! runs between and around them, keeping one run where two would meet.

use std::iter;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::Stmt;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::blanks::{blank_run_above, blank_run_below, whitespace_start_before},
    rules::RuleId,
    source::Source,
};

/// Returns each of `slots` beside the edit clearing its statement of
/// `body` per [`whole_line_deletions`], ascending by slot.
pub(crate) fn slot_deletions(
    source: &Source,
    body: &[Stmt],
    slots: impl IntoIterator<Item = usize>,
    rule: RuleId,
) -> impl Iterator<Item = (usize, Edit)> {
    let slots: Vec<usize> = slots.into_iter().sorted_unstable().collect();
    let edits = whole_line_deletions(source, slots.iter().map(|&slot| body[slot].range()), rule);
    slots.into_iter().zip(edits)
}

/// Returns one edit per range of `ranges` clearing its full lines, the
/// ranges ascending and each holding its lines alone. Ranges parted only
/// by blank rows clear as one block, each later member taking the blank
/// run above it. The block takes the narrower of the runs around it, the
/// one above on a tie, or the only one where it opens or closes its
/// notebook cell or module or where a `\` join holds the run above, and
/// a block whose last row has no line break also clears the break
/// closing the row above its run, unless a suppression of `rule` covers
/// that break.
fn whole_line_deletions(
    source: &Source,
    ranges: impl IntoIterator<Item = TextRange>,
    rule: RuleId,
) -> Vec<Edit> {
    let lines: Vec<TextRange> = ranges
        .into_iter()
        .map(|range| source.full_lines_within_cell(range))
        .collect();
    let rows = |run: TextRange| source.text().count_lines(run);
    lines
        .chunk_by(|upper, lower| {
            blank_run_above(source, *lower).is_some_and(|run| run.start() == upper.end())
        })
        .flat_map(|block| {
            let (&last, init) = block
                .split_last()
                .expect("invariant: a chunk holds a range");
            let (above, below) = match (
                blank_run_above(source, block[0]),
                blank_run_below(source, last),
            ) {
                (Some(above), Some(below)) if rows(below) < rows(above) => (None, Some(below)),
                (Some(above), _) => (Some(above), None),
                runs => runs,
            };
            let row_break = above
                .filter(|_| !source.slice(last).ends_with(['\n', '\r']))
                .map(|run| {
                    let at = whitespace_start_before(source, run.start());
                    TextRange::new(source.text().line_end(at), run.start())
                })
                .filter(|row_break| !source.suppression_map().suppresses(*row_break, rule));
            let start = row_break.map_or(above.unwrap_or(block[0]).start(), TextRange::start);
            iter::once(start)
                .chain(init.iter().map(Ranged::end))
                .chain(iter::once(below.unwrap_or(last).end()))
                .tuple_windows()
                .map(|(start, end)| Edit::deletion(start, end))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        rules::prune_inert_imports::PruneInertImports,
        testing::{applied_text, notebook, parse, range},
    };

    /// The rule whose suppressions the deletions read.
    const RULE: RuleId = PruneInertImports::SLUG;

    /// The text `source` reads once the statements at `slots` of its
    /// module body clear through [`whole_line_deletions`].
    fn cleared(source: &Source, slots: &[usize]) -> String {
        let body = &source.ast().body;
        let ranges = slots.iter().map(|&slot| body[slot].range());
        applied_text(source, whole_line_deletions(source, ranges, RULE))
    }

    #[rstest]
    #[case::alone_in_the_module("import a\n", &[0], "")]
    #[case::opening_the_module("import a\n\nx = 1\n", &[0], "x = 1\n")]
    #[case::closing_the_module("x = 1\n\nimport a\n", &[1], "x = 1\n")]
    #[case::narrower_run_below("x = 1\n\n\nimport a\n\ny = 2\n", &[1], "x = 1\n\n\ny = 2\n")]
    #[case::narrower_run_above("x = 1\n\nimport a\n\n\ny = 2\n", &[1], "x = 1\n\n\ny = 2\n")]
    #[case::no_run_above("import a\nimport b\n\nx = 1\n", &[1], "import a\n\nx = 1\n")]
    #[case::comment_bounding_the_run("# c\n\nimport a\n\n\nx = 1\n", &[0], "# c\n\n\nx = 1\n")]
    #[case::block_across_blank_rows(
        "x = 1\n\n\nimport a\n\nimport b\n\n\ny = 2\n",
        &[1, 2],
        "x = 1\n\n\ny = 2\n"
    )]
    #[case::block_on_adjacent_rows("x = 1\n\nimport a\nimport b\n\n\ny = 2\n", &[1, 2], "x = 1\n\n\ny = 2\n")]
    #[case::separate_blocks("import a\n\nx = 1\n\nimport b\n", &[0, 2], "x = 1\n")]
    #[case::join_holding_the_run_above("x = 1 \\\n\nimport a\n\ny = 2\n", &[1], "x = 1 \\\n\ny = 2\n")]
    #[case::unterminated_last_row("x = 1\n\nimport a", &[1], "x = 1")]
    #[case::unterminated_row_under_code("import os\nimport a", &[1], "import os")]
    #[case::crlf("import a\r\n\r\nx = 1\r\n", &[0], "x = 1\r\n")]
    #[case::crlf_unterminated("x = 1\r\n\r\nimport a", &[1], "x = 1")]
    #[case::under_a_byte_order_mark("\u{feff}import a\n\nx = 1\n", &[0], "\u{feff}x = 1\n")]
    #[case::unterminated_under_a_skipped_row("x = 1  # prose: skip\n\nimport a", &[1], "x = 1  # prose: skip\n")]
    fn whole_line_deletions_clear_the_narrower_blank_run_around_each_block(
        #[case] src: &str,
        #[case] slots: &[usize],
        #[case] expected: &str,
    ) {
        assert_eq!(cleared(&parse(src), slots), expected);
    }

    #[rstest]
    #[case::opening_a_cell(&["x = 1", "import a\n\ny = 2"], "x = 1\ny = 2\n")]
    #[case::closing_a_cell(&["x = 1\n\nimport a", "y = 2"], "x = 1\ny = 2\n")]
    #[case::alone_in_a_cell(&["y = 2", "import a"], "y = 2\n\n")]
    fn whole_line_deletions_hold_each_run_within_its_cell(
        #[case] cells: &[&str],
        #[case] expected: &str,
    ) {
        assert_eq!(cleared(&notebook(cells), &[1]), expected);
    }

    #[test]
    fn whole_line_deletions_take_the_run_above_on_a_tie() {
        let source = parse("x = 1\n\nimport a\n\ny = 2\n");
        let edits = whole_line_deletions(&source, [source.ast().body[1].range()], RULE);

        assert_eq!(edits, [Edit::range_deletion(range(6, 16))]);
    }
}

//! The statement windows a rule's edits fall inside.

use ruff_python_trivia::leading_indentation;
use ruff_text_size::TextRange;

use crate::{
    primitives::{range::merged_spans, splice::reparse_window},
    source::Source,
};

/// The leading whitespace of the last physical line `range` covers in
/// `text`, the indent the `Dedent` run past the window counts down from.
pub(super) fn closing_indent(text: &str, range: TextRange) -> &str {
    let slice = &text[range];
    leading_indentation(slice.rsplit_once('\n').map_or(slice, |(_, tail)| tail))
}

/// The statement windows covering `replaced`, ascending and merged
/// where two overlap or meet. `None` where an edit is covered by no
/// single statement, leaving the module itself as its window.
pub(super) fn covering(
    source: &Source,
    replaced: impl Iterator<Item = TextRange>,
) -> Option<Vec<TextRange>> {
    let module = source.module_range();
    let windows: Vec<TextRange> = replaced
        .map(|range| {
            let window = reparse_window(source, range);
            (window != module).then_some(window)
        })
        .collect::<Option<_>>()?;
    Some(merged_spans(windows))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{parse, range};

    #[rstest]
    #[case::a_single_line_statement("x = 1\n", range(0, 5), "")]
    #[case::a_block_closing_deeper("def f():\n    y = 2\n", range(0, 18), "    ")]
    #[case::a_block_closing_deeper_still(
        "if a:\n    if b:\n        c = 1\n",
        range(0, 29),
        "        "
    )]
    fn closing_indent_reads_the_last_line_of_the_window(
        #[case] text: &str,
        #[case] window: TextRange,
        #[case] expected: &str,
    ) {
        assert_eq!(closing_indent(text, window), expected);
    }

    #[test]
    fn covering_answers_none_where_an_edit_spans_two_statements() {
        let source = parse("x = 1\ny = 2\n");

        assert!(covering(&source, [range(0, 11)].into_iter()).is_none());
    }

    #[test]
    fn covering_folds_two_edits_inside_one_statement_into_one_window() {
        let source = parse("x = 1\ny = 2\n");

        assert_eq!(
            covering(&source, [range(0, 1), range(4, 5)].into_iter()),
            Some(vec![range(0, 5)]),
        );
    }

    #[rstest]
    #[case::two_statements_apart(&[(4, 5), (10, 11)], &[(0, 5), (6, 11)])]
    #[case::two_edits_in_one_statement(&[(0, 1), (4, 5)], &[(0, 5)])]
    #[case::edits_in_both_statements_out_of_order(&[(10, 11), (4, 5)], &[(0, 5), (6, 11)])]
    fn covering_merges_and_orders_the_windows_it_finds(
        #[case] edits: &[(u32, u32)],
        #[case] expected: &[(u32, u32)],
    ) {
        let source = parse("x = 1\ny = 2\n");
        let spans = edits.iter().map(|&(a, b)| range(a, b));
        let want: Vec<TextRange> = expected.iter().map(|&(a, b)| range(a, b)).collect();

        assert_eq!(covering(&source, spans), Some(want));
    }

    #[test]
    fn covering_narrows_to_the_statement_holding_each_edit() {
        let source = parse("x = 1\ny = 2\n");

        assert_eq!(
            covering(&source, [range(4, 5), range(10, 11)].into_iter()),
            Some(vec![range(0, 5), range(6, 11)]),
        );
    }

    #[test]
    fn covering_reaches_a_statement_nested_in_a_definition() {
        let source = parse("def f():\n    y = 2\n");

        assert_eq!(
            covering(&source, [range(17, 18)].into_iter()),
            Some(vec![range(13, 18)]),
        );
    }
}

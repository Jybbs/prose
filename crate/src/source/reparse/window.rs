//! The windows a rule's edits fall inside, each a statement or a run
//! of module-body siblings.

use ruff_python_ast::token::{Token, TokenKind};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::{range::merged_spans, splice::covering_window},
    source::Source,
};

/// A window's span in the buffer the source holds and the span the
/// woven text holds it at.
pub(super) struct Window {
    pub(super) held: TextRange,
    pub(super) slid: TextRange,
}

/// Ranges a window by where the woven text holds it, the buffer a
/// search over the written spans reads.
impl Ranged for Window {
    fn range(&self) -> TextRange {
        self.slid
    }
}

/// The levels `tokens`, a window's own, open on balance, each `Indent`
/// counting one up and each `Dedent` one down, so the window's end sits
/// that many levels past its start. The lexer tracks indentation on
/// logical lines alone, so a continuation line's whitespace never
/// reaches the count.
pub(super) fn net_indent(tokens: &[Token]) -> isize {
    tokens
        .iter()
        .map(|token| match token.kind() {
            TokenKind::Indent => 1,
            TokenKind::Dedent => -1,
            _ => 0,
        })
        .sum()
}

/// True for a token the lexer's indentation tracking reads a logical
/// line from, leaving out the trivia and the indent tokens it emits.
pub(super) fn is_code(kind: TokenKind) -> bool {
    !matches!(
        kind,
        TokenKind::Comment
            | TokenKind::Dedent
            | TokenKind::Indent
            | TokenKind::Newline
            | TokenKind::NonLogicalNewline
    )
}

/// The windows covering `replaced`, ascending and merged where two
/// overlap or meet: the innermost statement covering a range, or the
/// run of module-body siblings the range reaches where no single
/// statement covers it, a module-body window running on through the
/// gap after it so the reparse lexes the gap's line breaks against
/// what the window holds.
pub(super) fn covering(
    source: &Source,
    replaced: impl Iterator<Item = TextRange>,
) -> Vec<TextRange> {
    merged_spans(
        replaced
            .map(|range| {
                let window =
                    covering_window(source, range).unwrap_or_else(|| sibling_run(source, range));
                through_the_gap(source, window)
            })
            .collect(),
    )
}

/// True where `held` is a window of the module body rather than of a
/// statement nested in one, being a module-body sibling's own range or
/// a run of them, which the reparse may fill with any count of
/// statements.
pub(super) fn module_level(source: &Source, held: TextRange) -> bool {
    let body = &source.ast().body;
    body.iter().any(|stmt| stmt.range() == held) || covering_window(source, held).is_none()
}

/// The run of module-body siblings a range no single statement covers
/// reparses within, from the first sibling it overlaps, or the one
/// before it where it opens in a gap, or the module's start where none
/// precedes it, to the last sibling it overlaps, or the one after it
/// where it closes in a gap, or the module's end where none follows.
fn sibling_run(source: &Source, range: TextRange) -> TextRange {
    let body = &source.ast().body;
    let module = source.module_range();
    let first = body.partition_point(|stmt| stmt.end() <= range.start());
    let start = match body.get(first) {
        Some(stmt) if stmt.start() <= range.start() => stmt.start(),
        _ => first
            .checked_sub(1)
            .map_or(module.start(), |slot| body[slot].start()),
    };
    let past = body.partition_point(|stmt| stmt.start() < range.end());
    let end = match past.checked_sub(1) {
        Some(slot) if body[slot].end() >= range.end() => body[slot].end(),
        _ => body.get(past).map_or(module.end(), Ranged::end),
    };
    TextRange::new(start, end)
}

/// `window` run on to the next module-body sibling's start, or the
/// module's end, where it is a module-body window, and left as it is
/// where it is a statement nested in one.
fn through_the_gap(source: &Source, window: TextRange) -> TextRange {
    if !module_level(source, window) {
        return window;
    }
    let body = &source.ast().body;
    let next = body.partition_point(|stmt| stmt.start() < window.end());
    let end = body
        .get(next)
        .map_or(source.module_range().end(), Ranged::start);
    TextRange::new(window.start(), end)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{parse, range};

    #[rstest]
    #[case::a_single_line_statement("x = 1\n", range(0, 5), 0)]
    #[case::a_block_closing_deeper("def f():\n    y = 2\n", range(0, 18), 1)]
    #[case::a_block_closing_deeper_across_cr("def f():\r    y = 2\r", range(0, 18), 1)]
    #[case::a_block_closing_deeper_still("if a:\n    if b:\n        c = 1\n", range(0, 29), 2)]
    #[case::a_statement_closing_on_a_continuation_line("foo(\n        a)\n", range(0, 15), 0)]
    #[case::a_nested_statement_holding_no_newline("def f():\n    y = 2\n", range(13, 18), 0)]
    fn net_indent_counts_the_levels_a_window_opens(
        #[case] text: &str,
        #[case] window: TextRange,
        #[case] expected: isize,
    ) {
        let source = parse(text);

        assert_eq!(net_indent(source.tokens().in_range(window)), expected);
    }

    #[rstest]
    #[case::two_statements("x = 1\ny = 2\n", range(0, 11), range(0, 12))]
    #[case::a_blank_line_between_two_statements("x = 1\n\ny = 2\n", range(6, 6), range(0, 13))]
    #[case::ahead_of_the_first_statement("\nx = 1\n", range(0, 0), range(0, 7))]
    #[case::past_the_last_statement("x = 1\n", range(6, 6), range(0, 6))]
    fn covering_takes_the_sibling_run_where_no_statement_covers(
        #[case] text: &str,
        #[case] edit: TextRange,
        #[case] window: TextRange,
    ) {
        let source = parse(text);

        assert_eq!(covering(&source, [edit].into_iter()), vec![window]);
    }

    #[rstest]
    #[case::two_statements_apart(&[(4, 5), (10, 11)], &[(0, 12)])]
    #[case::two_edits_in_one_statement(&[(0, 1), (4, 5)], &[(0, 6)])]
    #[case::edits_in_both_statements_out_of_order(&[(10, 11), (4, 5)], &[(0, 12)])]
    fn covering_merges_and_orders_the_windows_it_finds(
        #[case] edits: &[(u32, u32)],
        #[case] expected: &[(u32, u32)],
    ) {
        let source = parse("x = 1\ny = 2\n");
        let spans = edits.iter().map(|&(a, b)| range(a, b));
        let want: Vec<TextRange> = expected.iter().map(|&(a, b)| range(a, b)).collect();

        assert_eq!(covering(&source, spans), want);
    }

    #[test]
    fn covering_reaches_a_statement_nested_in_a_definition() {
        let source = parse("def f():\n    y = 2\n");

        assert_eq!(
            covering(&source, [range(17, 18)].into_iter()),
            vec![range(13, 18)],
        );
    }
}

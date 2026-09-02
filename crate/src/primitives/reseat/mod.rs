//! Re-seats the continuation rows of a logical line once a rule removes
//! text from its rows, each row reading its indent against the bracket
//! it sits inside: a hanging row keeps its column, a row on a token's
//! column follows the token, and a row one or two indent steps past the
//! opener's row moves as that row does. A row inside a row-spanning
//! string, a row indented with a tab, and a row aligned to nothing keep
//! their columns.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::{LineRanges, OneIndexed, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    primitives::{
        INDENT_STEP,
        edit::insert_edit,
        inline::{display_width, indent_width},
        range::blocks_span,
        tokens::{is_closer, is_opener, open_brackets, tokens_within},
        travel::{frozen_rows, is_movable},
    },
    source::Source,
};

/// Emits into `edits` the indent deletions moving each continuation row
/// left by the columns the text it aligns to loses once `removals`,
/// the rule's own edits on the rows, apply. The reseat spans from the
/// start of the first row a removal touches to the end of its logical
/// line, and empty `removals` reseat nothing.
pub(crate) fn push_reseat_edits(source: &Source, removals: &[Edit], edits: &mut Vec<Edit>) {
    if removals.is_empty() {
        return;
    }
    let span = blocks_span(removals);
    let line = TextRange::new(
        source.text().line_start(span.start()),
        source.logical_line_tail(span.end()).end(),
    );
    let row_of = |offset: TextSize| source.line_index(offset);
    let removed = |offset: TextSize| removals.iter().any(|edit| edit.range().contains(offset));
    let tokens: Vec<&Token> = tokens_within(source, line)
        .filter(|token| !token.kind().is_trivia())
        .collect();
    // A removed token still brackets the rows inside it, whereas a row
    // aligns only to a token that survives on a row no removal joins to
    // the one above it.
    let anchors = |token: &Token| {
        !removed(token.start())
            && !removals.iter().any(|edit| {
                source.contains_line_break(edit.range())
                    && row_of(edit.end()) == row_of(token.start())
            })
    };
    let mut moved: FxHashMap<OneIndexed, usize> = FxHashMap::default();
    let token_move = |moved: &FxHashMap<OneIndexed, usize>, token: &Token| -> usize {
        let row = moved.get(&row_of(token.start())).copied().unwrap_or(0);
        let lost: usize = removals
            .iter()
            .filter(|edit| {
                row_of(edit.start()) == row_of(token.start()) && edit.end() <= token.start()
            })
            .map(|edit| {
                display_width(source.slice(edit.range())) - edit.content().map_or(0, display_width)
            })
            .sum();
        row + lost
    };
    let frozen = frozen_rows(source, line);
    for (row, line_text) in source.slice(line).universal_newlines().enumerate() {
        let start = line.start() + line_text.start();
        if !is_movable(row, line_text.as_str(), &frozen) || removed(start) {
            continue;
        }
        if leading_indentation(line_text.as_str()).contains('\t') {
            continue;
        }
        let indent = indent_width(line_text.as_str());
        let Some(opener) = open_brackets(
            tokens
                .iter()
                .copied()
                .take_while(|token| token.start() < start),
        )
        .pop() else {
            continue;
        };
        let opener_row = row_of(opener);
        let after: Vec<&Token> = tokens
            .iter()
            .copied()
            .filter(|token| anchors(token))
            .filter(|token| token.start() > opener && row_of(token.start()) == opener_row)
            .collect();
        let hangs_from_row = moved.get(&opener_row).copied().unwrap_or(0);
        let at = |token: &Token| source.column_of(token.start()) == indent;
        let shift = match after.first() {
            None => hangs_from_row,
            Some(first) if at(first) => token_move(&moved, first),
            Some(_) => {
                let row_indent = source.line_indent_width(opener);
                if let Some(code) = after
                    .iter()
                    .find(|token| at(token) && is_code(token.kind()))
                {
                    token_move(&moved, code)
                } else if indent == row_indent + INDENT_STEP
                    || indent == row_indent + 2 * INDENT_STEP
                {
                    hangs_from_row
                } else if let Some(token) = after.iter().find(|token| at(token)) {
                    token_move(&moved, token)
                } else {
                    0
                }
            }
        };
        moved.insert(row_of(start), shift);
        let taken = shift.min(indent);
        if taken > 0 {
            let taken = TextSize::try_from(taken).expect("indent fits u32");
            insert_edit(edits, Edit::range_deletion(TextRange::at(start, taken)));
        }
    }
}

/// True for a token a row aligns to by intent rather than by the
/// coincidence of a hang, every kind but a bracket and a comma.
fn is_code(kind: TokenKind) -> bool {
    !is_opener(kind) && !is_closer(kind) && kind != TokenKind::Comma
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, parse};

    /// Each two-space run `src` writes directly after a `(` or a `[`,
    /// the padding a shedding rule removes, ascending by start.
    fn opener_padding(src: &str) -> Vec<TextRange> {
        let mut pads: Vec<TextRange> = src
            .match_indices("(  ")
            .chain(src.match_indices("[  "))
            .map(|(at, _)| {
                TextRange::new(
                    TextSize::try_from(at + 1).expect("offset fits u32"),
                    TextSize::try_from(at + 3).expect("offset fits u32"),
                )
            })
            .collect();
        pads.sort_by_key(Ranged::start);
        pads
    }

    /// The text `src` leaves once its opener padding is deleted and the
    /// reseat edits that deletion earns are applied beside it.
    fn reseated(src: &str) -> String {
        let source = parse(src);
        let removals: Vec<Edit> = opener_padding(src)
            .into_iter()
            .map(Edit::range_deletion)
            .collect();
        let mut edits = Vec::new();
        push_reseat_edits(&source, &removals, &mut edits);
        for removal in removals {
            insert_edit(&mut edits, removal);
        }
        applied_text(&source, edits)
    }

    #[rstest]
    #[case::hangs_one_step_past_the_opener_row(
        "result = compute(  alpha,\n    beta,\n)\n",
        "result = compute(alpha,\n    beta,\n)\n"
    )]
    #[case::hangs_two_steps_past_the_opener_row(
        "result = compute(  alpha,\n        beta,\n)\n",
        "result = compute(alpha,\n        beta,\n)\n"
    )]
    #[case::follows_the_comma_it_sits_under(
        "result = compute(  alpha, beta,\n                        gamma,\n)\n",
        "result = compute(alpha, beta,\n                      gamma,\n)\n"
    )]
    #[case::holds_a_row_aligned_to_nothing(
        "result = compute(  alpha,\n      beta,\n)\n",
        "result = compute(alpha,\n      beta,\n)\n"
    )]
    #[case::follows_the_name_it_sits_under(
        "result = outer([  alpha,\n                 inner(  beta,\n                         gamma,\n                 ),\n])\n",
        "result = outer([alpha,\n                 inner(beta,\n                       gamma,\n                 ),\n])\n"
    )]
    #[case::holds_a_tab_indented_row(
        "result = compute(  alpha, beta,\n\t\t\t\t\t\t\tgamma,\n)\n",
        "result = compute(alpha, beta,\n\t\t\t\t\t\t\tgamma,\n)\n"
    )]
    fn a_continuation_row_reseats_against_the_token_it_reads(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(reseated(src), expected);
    }

    #[test]
    fn empty_removals_reseat_nothing() {
        let mut edits = Vec::new();
        push_reseat_edits(&parse("x = compute(\n    1,\n)\n"), &[], &mut edits);
        assert!(edits.is_empty());
    }
}

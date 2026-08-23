//! Re-seats the continuation rows of a logical line once a rule removes
//! text from its rows, each row reading its indent against the bracket
//! it sits inside. A row hanging below an opener that ends its row keeps
//! its column, a row indented to the column directly past its bracket's
//! opener tracks the first token after that opener, a row indented to a
//! code token on the opener's row follows the token, a row one or two
//! indent steps past the opener's row hangs from that row and moves as
//! it does, and a row on any other token's column follows the token. A
//! row inside a row-spanning string, a row indented with a tab, and a
//! row aligned to nothing keep their columns.

use std::collections::BTreeMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::OneIndexed;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{
        INDENT_STEP,
        edit::insert_edit,
        inline::indent_width,
        tokens::{is_closer, is_opener},
        travel::frozen_rows,
    },
    source::Source,
};

/// Emits into `edits` the indent deletions moving each continuation row
/// of `line` left by the columns the text it aligns to loses once
/// `removals`, the rule's own edits on the line's rows, apply. `line`
/// runs from the start of the first row a removal touches to the end of
/// the logical line.
pub(crate) fn push_reseat_edits(
    source: &Source,
    line: TextRange,
    removals: &[Edit],
    edits: &mut Vec<Edit>,
) {
    let row_of = |offset: TextSize| source.line_index(offset);
    let removed = |offset: TextSize| removals.iter().any(|edit| edit.range().contains(offset));
    let tokens: Vec<&Token> = source
        .tokens_overlapping(line)
        .filter(|token| line.contains(token.start()) && !token.kind().is_trivia())
        .collect();
    // A removed token still brackets the rows inside it, whereas a row
    // aligns only to a token that survives on a row no removal joins to
    // the one above it.
    let anchors = |token: &Token| {
        !removed(token.start())
            && !removals.iter().any(|edit| {
                source.contains_line_break(edit.range())
                    && row_of(edit.range().end()) == row_of(token.start())
            })
    };
    let mut moved: BTreeMap<OneIndexed, usize> = BTreeMap::new();
    let token_move = |moved: &BTreeMap<OneIndexed, usize>, token: &Token| -> usize {
        let row = moved.get(&row_of(token.start())).copied().unwrap_or(0);
        let lost: usize = removals
            .iter()
            .filter(|edit| {
                row_of(edit.range().start()) == row_of(token.start())
                    && edit.range().end() <= token.start()
            })
            .map(|edit| {
                source.slice(edit.range()).width()
                    - edit.content().map_or(0, UnicodeWidthStr::width)
            })
            .sum();
        row + lost
    };
    let frozen = frozen_rows(source, line);
    let mut row_start = line.start();
    for (row, line_text) in source.slice(line).split_inclusive('\n').enumerate() {
        let start = row_start;
        row_start += line_text.text_len();
        if row == 0 || frozen.get(row) == Some(&true) || removed(start) {
            continue;
        }
        let indentation = leading_indentation(line_text);
        if indentation.contains('\t') || line_text.trim().is_empty() {
            continue;
        }
        let indent = indent_width(line_text);
        let Some(opener) = enclosing_opener(&tokens, start) else {
            continue;
        };
        let opener_row = row_of(opener.start());
        let after: Vec<&Token> = tokens
            .iter()
            .copied()
            .filter(|token| anchors(token))
            .filter(|token| token.start() > opener.start() && row_of(token.start()) == opener_row)
            .collect();
        let hangs_from_row = moved.get(&opener_row).copied().unwrap_or(0);
        let at = |token: &Token| source.column_of(token.start()) == indent;
        let shift = match after.first() {
            None => hangs_from_row,
            Some(first) if at(first) => token_move(&moved, first),
            Some(_) => {
                let row_indent = source.line_indent_width(opener.start());
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

/// The opener of the innermost bracket still open at `offset` among
/// `tokens`, `None` where none is.
fn enclosing_opener<'t>(tokens: &[&'t Token], offset: TextSize) -> Option<&'t Token> {
    let mut pending = 0_usize;
    for token in tokens.iter().rev().filter(|token| token.start() < offset) {
        if is_closer(token.kind()) {
            pending += 1;
        } else if is_opener(token.kind()) {
            if pending == 0 {
                return Some(token);
            }
            pending -= 1;
        }
    }
    None
}

/// True for a token a row aligns to by intent rather than by the
/// coincidence of a hang, every kind but a bracket and a comma.
fn is_code(kind: TokenKind) -> bool {
    !is_opener(kind) && !is_closer(kind) && kind != TokenKind::Comma
}

//! Re-seats the continuation rows of a logical line once a rule removes
//! text from its rows, each row's indent read against the bracket it
//! sits inside: a hanging row keeps its column, a row on a token's
//! column follows the token, and a row one or two indent steps past the
//! opener's row moves as that row does. A row inside a row-spanning
//! string, a row indented with a tab, and a row aligned to nothing keep
//! their columns. [`row_move`] writes the edit moving one row, and
//! [`RowMoves`] files the rows a rule's values carry into the fix groups
//! moving them.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::Token;
use ruff_python_trivia::leading_indentation;
use ruff_source_file::{LineRanges, OneIndexed, UniversalNewlines};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    primitives::{
        edit::insert_edit,
        inline::{display_width, indent_width},
        range::blocks_span,
        tokens::tokens_within,
        travel::{Seat, frozen_rows, is_movable, seat},
    },
    source::Source,
};

/// The continuation rows a rule's values carry, each row's line start
/// mapped to the fix group its move joins and the columns it moves. The
/// group is that of the first value carrying the row, and the columns
/// sum across every value that carries it.
#[derive(Default)]
pub(crate) struct RowMoves(FxHashMap<TextSize, (usize, isize)>);

impl RowMoves {
    /// Moves each of `rows` a further `shift` columns, a row no earlier
    /// value carries joining fix group `group`.
    pub(crate) fn carry(&mut self, group: usize, rows: &[TextRange], shift: isize) {
        for row in rows {
            self.0.entry(row.start()).or_insert((group, 0)).1 += shift;
        }
    }

    /// Appends the [`row_move`] of each carried row to the group in
    /// `groups` its move joins, leaving out a row whose shifts cancel.
    pub(crate) fn file_into(self, source: &Source, groups: &mut [Vec<Edit>]) {
        for (line, (group, shift)) in self.0 {
            groups[group].extend(row_move(source, line, shift));
        }
    }

    /// Returns the columns the row opening at `line` moves, zero where
    /// no value carries it.
    pub(crate) fn shift(&self, line: TextSize) -> isize {
        self.0.get(&line).map_or(0, |&(_, shift)| shift)
    }
}

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
        let Some(seat) = seat(source, &tokens, anchors, start, indent) else {
            continue;
        };
        let shift = match seat {
            Seat::Token(token) => token_move(&moved, token),
            Seat::Hangs(row) => moved.get(&row).copied().unwrap_or(0),
            Seat::Free => 0,
        };
        moved.insert(row_of(start), shift);
        if let Some(edit) = row_move(source, start, -shift.cast_signed()) {
            insert_edit(edits, edit);
        }
    }
}

/// Returns the edit moving the row opening at `line` by `shift` columns,
/// spaces inserted where its content starts or deleted from the line's
/// start, a row moving left giving up at most the indent it holds.
/// `None` where the row stays.
pub(crate) fn row_move(source: &Source, line: TextSize, shift: isize) -> Option<Edit> {
    let indent = leading_indentation(source.slice(source.row_tail(line)));
    match usize::try_from(shift) {
        Ok(0) => None,
        Ok(columns) => Some(Edit::insertion(
            " ".repeat(columns),
            line + indent.text_len(),
        )),
        Err(_) => {
            let taken = shift.unsigned_abs().min(indent.len());
            (taken > 0).then(|| {
                let taken = TextSize::try_from(taken).expect("an indent fits u32");
                Edit::range_deletion(TextRange::at(line, taken))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, parse, range};

    /// Each two-space run in `src` directly after a `(` or a `[`, the
    /// padding a shedding rule removes, ascending by start.
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

    /// `src` with its opener padding deleted and the reseat edits that
    /// deletion produces applied beside it.
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

    #[rstest]
    #[case::right("x = f(a,\n      b)\n", 2, "x = f(a,\n        b)\n")]
    #[case::left("x = f(a,\n      b)\n", -2, "x = f(a,\n    b)\n")]
    #[case::past_its_indent("x = f(a,\n  b)\n", -4, "x = f(a,\nb)\n")]
    fn row_move_shifts_a_row_by_at_most_its_indent_leftward(
        #[case] src: &str,
        #[case] shift: isize,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let edit = row_move(&source, TextSize::new(9), shift).expect("the row moves");
        assert_eq!(applied_text(&source, vec![edit]), expected);
    }

    #[rstest]
    #[case::still("x = f(a,\n      b)\n", 0)]
    #[case::left_of_an_empty_indent("x = f(a,\nb)\n", -2)]
    fn row_move_leaves_a_row_that_stays(#[case] src: &str, #[case] shift: isize) {
        assert!(row_move(&parse(src), TextSize::new(9), shift).is_none());
    }

    #[test]
    fn row_moves_file_a_row_with_the_first_group_by_its_summed_shift() {
        let source = parse("x = f(a,\n      b,\n      c)\n");
        let (b, c) = (range(9, 17), range(18, 26));
        let mut moves = RowMoves::default();
        moves.carry(0, &[b, c], 2);
        moves.carry(1, &[b], 1);
        moves.carry(1, &[c], -2);
        assert_eq!((moves.shift(b.start()), moves.shift(c.start())), (3, 0));
        let mut groups = vec![Vec::new(), Vec::new()];
        moves.file_into(&source, &mut groups);
        assert!(groups[1].is_empty());
        assert_eq!(
            applied_text(&source, groups.concat()),
            "x = f(a,\n         b,\n      c)\n"
        );
    }
}

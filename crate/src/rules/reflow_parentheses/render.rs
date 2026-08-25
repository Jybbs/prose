//! Whether a pair breaks and the text it lands as, its brackets alone
//! on their rows and every operand led by the operator joining it to
//! the row above, or its interior whole on one row where that fits.
//! Each operand renders through the pairs this same pass sheds inside
//! it, so one edit carries both directions.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use super::{
    Shedder,
    chain::{Operand, operands},
    plan::{Candidate, breaks_held_inside},
};
use crate::{
    primitives::{
        edit::{apply_inline_edits, insert_edit},
        fracture::outermost,
        inline::folded_line_form,
        layout::{Separator, explode_parens, item_indent},
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener},
        travel::hung_block_through,
    },
    source::Source,
};

impl Shedder<'_> {
    /// True where a bracket this pass leaves standing still holds
    /// `pair` open, so the construct that bracket belongs to lays out
    /// the rows around it. A grouping pair the pass sheds holds nothing,
    /// leaving the reading the same before and after the shed.
    fn held_inside_a_bracket(&self, pair: TextRange, candidates: &[Candidate]) -> bool {
        let head = self.source.logical_line_start(pair.start());
        let mut open: Vec<TextSize> = Vec::new();
        for token in self
            .source
            .tokens_overlapping(head)
            .filter(|token| head.contains(token.start()))
        {
            if is_opener(token.kind()) {
                open.push(token.start());
            } else if is_closer(token.kind()) {
                open.pop();
            }
        }
        open.iter().any(|start| {
            !candidates
                .iter()
                .any(|other| other.sheds && other.pair.start() == *start)
        })
    }

    /// True where every line break inside `inner` sits within a bracket
    /// `inner` opens and this pass leaves standing, so the pair around
    /// it holds none of them and the bracket that does lays out the
    /// rows. A bracket `nested` takes out holds nothing, leaving the
    /// reading the same before and after those pairs shed. An interior
    /// carrying no break reads false, the pair being free to open rows
    /// of its own.
    fn holds_its_breaks_inside(&self, inner: TextRange, nested: &[Edit]) -> bool {
        let inside_a_shed =
            |range: TextRange| nested.iter().any(|edit| edit.range().contains_range(range));
        self.source.contains_line_break(inner)
            && breaks_held_inside(self.source, inner, &inside_a_shed)
    }

    /// The paren removals every candidate `candidate` encloses earns,
    /// ascending by start, the text a break renders its operands
    /// through. A candidate whose own removal shifts the parse keeps
    /// its pair and contributes nothing.
    fn nested_shed_edits(&self, candidate: &Candidate, candidates: &[Candidate]) -> Vec<Edit> {
        let edits: Vec<Edit> = candidates
            .iter()
            .filter(|other| other.sheds && candidate.inner.contains_range(other.pair))
            .flat_map(Candidate::paren_removals)
            .collect();
        outermost(edits)
    }

    /// The indent of the row `pair`'s own statement opens on, the seat
    /// a break hangs its rows from. The walk skips the trivia ahead of
    /// that statement, so a comment or a blank row between two
    /// statements leaves the seat reading the statement's own row
    /// rather than the row the `(` happens to sit on.
    fn statement_indent(&self, pair: TextRange) -> usize {
        let head = self.source.logical_line_start(pair.start());
        let opens_at = self
            .source
            .tokens_overlapping(head)
            .find(|token| head.contains(token.start()) && !token.kind().is_trivia())
            .map_or(pair.start(), Ranged::start);
        self.source.line_indent_width(opens_at)
    }

    /// Emits the replacement breaking `candidate`'s pair across rows,
    /// reporting whether the break owns the pair's shape. The interior
    /// holds one row of its own where its joined form fits a row one
    /// indent step in, and takes one row per operand otherwise. Every
    /// row renders through the pairs `candidate` encloses that this
    /// pass sheds, so the division and the text both read what the
    /// pass leaves rather than what it was handed.
    pub(super) fn push_break_edits(
        &mut self,
        candidate: &Candidate,
        candidates: &[Candidate],
    ) -> bool {
        let Candidate { inner, pair, .. } = *candidate;
        if candidate.links || self.held_inside_a_bracket(pair, candidates) {
            return false;
        }
        let nested = self.nested_shed_edits(candidate, candidates);
        if self.holds_its_breaks_inside(inner, &nested) {
            return false;
        }
        let sheds_pair = |range: TextRange| nested.iter().any(|edit| edit.start() == range.start());
        let Some(chain) = operands(self.source, candidate.expr, &sheds_pair) else {
            return false;
        };
        let indent = self.statement_indent(pair);
        let item = item_indent(indent);
        let joined = apply_inline_edits(self.source, inner, &nested);
        let text = match folded_line_form(self.source, candidate.expr, &joined)
            .filter(|row| item + row.width() <= self.code_line_length)
        {
            Some(row) => wrapped(self.source, &row, indent),
            None => broken(self.source, &chain, &nested, indent),
        };
        if !splice_preserves_tree(self.source, pair, &text) {
            return false;
        }
        // The replacement spans the whole pair so it covers every
        // nested shed, which `outermost` then drops rather than leaving
        // to collide with it. A pair the source already wrote in this
        // shape earns no edit and still answers for its own layout.
        if text != self.source.slice(pair) {
            insert_edit(&mut self.edits, Edit::range_replacement(text, pair));
        }
        true
    }
}

/// The pair holding `chain` rewritten across rows, its `(` closing the
/// opening row, each operand on a row of its own one indent step past
/// `indent` behind the operator joining it to the row above, and its
/// `)` opening the row back at `indent`. Each operand renders through
/// `nested`.
fn broken(source: &Source, chain: &[Operand], nested: &[Edit], indent: usize) -> String {
    let item = item_indent(indent);
    explode_parens(
        source.newline_str(),
        indent,
        chain.len(),
        |out, row| {
            let Operand { lead, range } = chain[row];
            if let Some(operator) = lead {
                out.push_str(operator);
                out.push(' ');
            }
            out.push_str(&hung_block_through(source, range, nested, item));
        },
        Separator::None,
    )
}

/// The pair holding `row` whole on one row of its own, one indent step
/// past `indent`, with its brackets opening and closing the rows around
/// it.
fn wrapped(source: &Source, row: &str, indent: usize) -> String {
    explode_parens(
        source.newline_str(),
        indent,
        1,
        |out, _| out.push_str(row),
        Separator::None,
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        rules::reflow_parentheses::chain::operands,
        testing::{first_expr, parse},
    };

    #[rstest]
    #[case("a and b", "(\n    a\n    and b\n)")]
    #[case("a and b and c", "(\n    a\n    and b\n    and c\n)")]
    #[case("a is not b", "(\n    a\n    is not b\n)")]
    #[case("a not  in b", "(\n    a\n    not in b\n)")]
    #[case("a + b * c", "(\n    a\n    + b * c\n)")]
    fn broken_leads_each_row_with_its_operator(#[case] src: &str, #[case] expected: &str) {
        let source = parse(src);
        let expr = first_expr(&source);
        let holds = |_: TextRange| false;
        let chain = operands(&source, expr, &holds).expect("the source holds a chain");
        assert_eq!(broken(&source, &chain, &[], 0), expected);
    }
}

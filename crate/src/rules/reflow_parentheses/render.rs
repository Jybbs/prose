//! Whether a pair breaks and the text it lands as, its brackets alone
//! on their rows and every operand led by the operator joining it to
//! the row above, or its interior whole on one row where that fits.
//! Each operand renders through the pairs this same pass sheds inside
//! it.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};

use super::{
    Shedder,
    chain::{Operand, operands},
    plan::{Candidate, breaks_held_inside, shedding_inside},
};
use crate::{
    primitives::{
        edit::{apply_inline_edits, insert_edit},
        fracture::outermost,
        inline::{display_width, folded_line_form},
        layout::{Separator, explode_parens, item_indent},
        splice::splice_preserves_tree,
        tokens::{open_brackets, tokens_within},
        travel::hung_block_through,
    },
    source::Source,
};

impl Shedder<'_> {
    /// True where a bracket this pass leaves standing still holds `pair`
    /// open.
    fn held_inside_a_bracket(&self, pair: TextRange, candidates: &[Candidate]) -> bool {
        let head = self.source.logical_line_start(pair.start());
        open_brackets(tokens_within(self.source, head))
            .iter()
            .any(|start| {
                candidates
                    .binary_search_by_key(start, |other| other.pair.start())
                    .ok()
                    .is_none_or(|found| !candidates[found].sheds)
            })
    }

    /// True where every line break inside `inner` sits within a bracket
    /// `inner` opens and this pass leaves standing, and false for an
    /// interior carrying no break.
    fn holds_its_breaks_inside(&self, inner: TextRange, nested: &[Edit]) -> bool {
        let inside_a_shed =
            |range: TextRange| nested.iter().any(|edit| edit.range().contains_range(range));
        self.source.contains_line_break(inner)
            && breaks_held_inside(self.source, inner, &inside_a_shed)
    }

    /// The indent of the row `pair`'s own statement opens on, the seat a
    /// break hangs its rows from, read past the trivia ahead of that
    /// statement.
    fn statement_indent(&self, pair: TextRange) -> usize {
        let head = self.source.logical_line_start(pair.start());
        let opens_at = self
            .source
            .first_token_offset_in_range(head, |token| !token.kind().is_trivia())
            .unwrap_or(pair.start());
        self.source.line_indent_width(opens_at)
    }

    /// Emits the replacement breaking `candidate`'s pair across rows,
    /// reporting whether the break owns the pair's shape. The interior
    /// holds one row of its own where its joined form fits a row one
    /// indent step in, and takes one row per operand otherwise. Every row
    /// renders through the pairs `candidate` encloses that this pass sheds.
    pub(super) fn push_break_edits(
        &mut self,
        candidate: &Candidate,
        candidates: &[Candidate],
    ) -> bool {
        let Candidate { inner, pair, .. } = *candidate;
        if candidate.links || self.held_inside_a_bracket(pair, candidates) {
            return false;
        }
        let nested = nested_shed_edits(candidate, candidates);
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
            .filter(|row| item + display_width(row) <= self.code_line_length)
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
/// The paren removals every candidate `candidate` encloses earns,
/// ascending by start, the text a break renders its operands through.
/// A candidate whose own removal shifts the parse keeps its pair and
/// contributes nothing.
fn nested_shed_edits(candidate: &Candidate, candidates: &[Candidate]) -> Vec<Edit> {
    outermost(
        shedding_inside(candidate.inner, candidates)
            .flat_map(Candidate::paren_removals)
            .collect(),
    )
}

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

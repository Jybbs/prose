//! Whether a pair breaks and the text it lands as, its brackets alone
//! on their rows and every operand led by the operator joining it to
//! the row above, or its interior whole on one row where that fits.
//! Each operand renders through the pairs this same pass sheds inside
//! it, and joins its own rows where it fits one.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    Shedder,
    chain::{Operand, operands},
    measure::fold_edits,
    plan::{Candidate, break_depths, breaks_held_inside, shedding_inside},
};
use crate::{
    primitives::{
        edit::{apply_inline_edits, insert_edit},
        fracture::outermost,
        inline::{display_width, folded_line_form, spans_rows},
        layout::{Separator, explode_parens, item_indent},
        slots::starting_within,
        splice::splice_preserves_tree,
        tokens::{open_brackets, tokens_within},
        travel::hung_block_through,
    },
    source::Source,
};

impl Shedder<'_> {
    /// True where a bracket this pass leaves standing still holds `pair`
    /// open, a bracket being a pair the source wrote rather than one the
    /// break adds.
    fn held_inside_a_bracket(&self, pair: TextRange, candidates: &[Candidate]) -> bool {
        let head = self.source.logical_line_start(pair.start());
        open_brackets(tokens_within(self.source, head))
            .iter()
            .any(|&start| {
                !starting_within(
                    candidates,
                    TextRange::at(start, TextSize::of('(')),
                    |other| other.pair.start(),
                )
                .any(|other| !other.adds() && other.sheds)
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

    /// The edits closing the soft breaks inside each operand of `chain`
    /// that joins onto one row, read through `nested`. An operand joins
    /// where no break inside it sits within a bracket it opens and its
    /// joined form fits behind its operator one indent step past
    /// `indent`.
    fn operand_folds(&self, chain: &[Operand], nested: &[Edit], indent: usize) -> Vec<Edit> {
        let item = item_indent(indent);
        chain
            .iter()
            .filter(|operand| {
                break_depths(self.source, operand.range, &|_| false).all(|depth| depth == 0)
            })
            .flat_map(|operand| {
                let folds = fold_edits(self.source, operand.range);
                let row = apply_inline_edits(
                    self.source,
                    operand.range,
                    &outermost([nested, &folds].concat()),
                );
                let lead = operand.lead.map_or(0, |operator| operator.len() + 1);
                let fits = item + lead + display_width(&row) <= self.code_line_length;
                (fits && !spans_rows(&row)).then_some(folds)
            })
            .flatten()
            .collect()
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

    /// The text breaking `candidate`'s pair across rows, `None` where a
    /// break does not own the pair's shape. The interior holds one row of
    /// its own where its joined form fits a row one indent step in, unless
    /// `candidate` stacks, and takes one row per operand otherwise, each
    /// operand joining its own rows where [`Self::operand_folds`] holds.
    /// Every row renders through the pairs `candidate` encloses that this
    /// pass sheds.
    pub(super) fn broken_form(
        &self,
        candidate: &Candidate,
        candidates: &[Candidate],
    ) -> Option<String> {
        let Candidate { inner, pair, .. } = *candidate;
        if candidate.links || self.held_inside_a_bracket(pair, candidates) {
            return None;
        }
        let nested = nested_shed_edits(candidate, candidates);
        if self.holds_its_breaks_inside(inner, &nested) {
            return None;
        }
        // A pair the break adds folds the soft breaks inside it, each of
        // which sits in a pair the source wrote.
        let nested = if candidate.adds() {
            outermost([nested, fold_edits(self.source, inner)].concat())
        } else {
            nested
        };
        let sheds_pair = |range: TextRange| nested.iter().any(|edit| edit.start() == range.start());
        let chain = operands(self.source, candidate.expr, &sheds_pair)?;
        let indent = self.statement_indent(pair);
        let item = item_indent(indent);
        let joined = apply_inline_edits(self.source, inner, &nested);
        let text = match folded_line_form(self.source, candidate.expr, &joined)
            .filter(|row| !candidate.stacks && item + display_width(row) <= self.code_line_length)
        {
            Some(row) => wrapped(self.source, &row, indent),
            None => {
                let folds = self.operand_folds(&chain, &nested, indent);
                broken(
                    self.source,
                    &chain,
                    &outermost([nested, folds].concat()),
                    indent,
                )
            }
        };
        splice_preserves_tree(self.source, pair, &text).then_some(text)
    }

    /// Emits the replacement [`Self::broken_form`] writes for
    /// `candidate`'s pair, reporting whether the break owns the pair's
    /// shape.
    pub(super) fn push_break_edits(
        &mut self,
        candidate: &Candidate,
        candidates: &[Candidate],
    ) -> bool {
        let Some(text) = self.broken_form(candidate, candidates) else {
            return false;
        };
        // The replacement spans the whole pair and covers every nested
        // shed, which `outermost` then drops. A pair the source already
        // wrote in this shape earns no edit and still reports `true`.
        if text != self.source.slice(candidate.pair) {
            insert_edit(
                &mut self.edits,
                Edit::range_replacement(text, candidate.pair),
            );
        }
        true
    }
}

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
    #[case("a if b else c", "(\n    a\n    if b\n    else c\n)")]
    fn broken_leads_each_row_with_its_operator(#[case] src: &str, #[case] expected: &str) {
        let source = parse(src);
        let expr = first_expr(&source);
        let holds = |_: TextRange| false;
        let chain = operands(&source, expr, &holds).expect("the source holds a chain");
        assert_eq!(broken(&source, &chain, &[], 0), expected);
    }
}

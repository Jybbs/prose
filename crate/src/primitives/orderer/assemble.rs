//! Assembles reordered member blocks back into source text, the
//! cell-edit form included.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};

use super::*;
use crate::{
    primitives::{
        edit::{any_owned, narrowed_replacement},
        range::blocks_span,
        slots::slot_runs,
    },
    source::Source,
};

/// The member blocks of one body, the text rendered for each, and the
/// slot order they assemble in, the three every assembly reads
/// together. A rewriting rule builds one through
/// [`rendered_member_blocks`] and permutes `order` before assembling.
pub(crate) struct Assembly<'src> {
    pub(crate) blocks: Vec<TextRange>,
    pub(crate) order: Vec<usize>,
    pub(crate) rendered: Vec<Cow<'src, str>>,
}

impl<'src> Assembly<'src> {
    /// True where the assembly reproduces the source it was read from,
    /// nothing forcing a rewrite, no block re-rendered, and the order
    /// the identity.
    fn reproduces_source(&self, forced: bool) -> bool {
        !forced && !any_owned(&self.rendered) && is_identity(&self.order)
    }

    /// One fix group per notebook cell the blocks span and one for an
    /// ordinary module, each holding the [`piecewise_edits`] of its own
    /// slots. `order` never crosses a cell boundary, and a group whose
    /// pieces all match the source is dropped.
    pub(crate) fn cell_edits(
        &self,
        source: &'src Source,
        forced: bool,
        mut gap: impl FnMut(usize) -> Option<&'src str>,
    ) -> Vec<Vec<Edit>> {
        if !source.is_notebook() && self.reproduces_source(forced) {
            return Vec::new();
        }
        slot_runs(&self.blocks, |a, b| source.same_cell(a.start(), b.start()))
            .filter_map(|run| {
                let edits = piecewise_edits(
                    source,
                    &self.blocks,
                    &self.rendered,
                    &self.order,
                    run,
                    &mut gap,
                );
                (!edits.is_empty()).then_some(edits)
            })
            .collect()
    }

    /// The assembled text with `gap`, returned alongside the span it
    /// covers. Short-circuits to a borrow of the source span when no
    /// child rewrote and `order` is identity, unless `forced` holds, the
    /// signal a gap override reshapes spacing without reordering.
    pub(crate) fn or_borrow(
        &self,
        source: &'src Source,
        forced: bool,
        gap: impl FnMut(usize) -> Option<&'src str>,
    ) -> (Cow<'src, str>, TextRange) {
        let span = blocks_span(&self.blocks);
        if self.reproduces_source(forced) {
            return (Cow::Borrowed(source.slice(span)), span);
        }
        (
            Cow::Owned(assemble_blocks(
                source,
                &self.blocks,
                &self.rendered,
                &self.order,
                gap,
            )),
            span,
        )
    }
}

/// Splices each rendered child at its sorted position. `gap_override`
/// returning `Some(text)` for new-order slot `i` substitutes that
/// text for the source gap between slot `i` and slot `i + 1`. A
/// `None` return copies the source gap verbatim. `blocks` must be
/// non-empty and in source order, with `rendered` and `order` the
/// same length as `blocks`.
pub(crate) fn assemble_blocks<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
    mut gap_override: impl FnMut(usize) -> Option<&'src str>,
) -> String {
    let mut out = String::with_capacity(blocks_span(blocks).len().to_usize());
    walk_assembly(
        source,
        blocks,
        rendered,
        order,
        0..blocks.len(),
        &mut gap_override,
        |_, text| out.push_str(text),
    );
    out
}

/// Reorders sibling members by `classify`, the separators kept in the
/// verbatim gaps between bare member spans, `render_block` rewriting each
/// member's slice. Returns the rewritten text and the span it covers. A
/// multi-line group whose members carry trailing comments uses
/// `reorder_separated` instead.
pub(crate) fn reorder_text<'src, 'a, T, S, F>(
    source: &'src Source,
    items: &'a [T],
    classify: impl FnMut(&'a T) -> Option<S>,
    mut render_block: F,
) -> (Cow<'src, str>, TextRange)
where
    T: Ranged,
    S: Ord,
    F: FnMut(usize, TextRange) -> Cow<'src, str>,
{
    if items.is_empty() {
        return (Cow::Borrowed(""), TextRange::default());
    }
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'src, str>>) = items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let block = t.range();
            (block, render_block(i, block))
        })
        .unzip();
    let mut assembly = Assembly {
        blocks,
        order: (0..items.len()).collect(),
        rendered,
    };
    permute_full(&mut assembly.order, items, classify);
    assembly.or_borrow(source, false, |_| None)
}

/// One narrowed edit per piece of [`walk_assembly`] that differs from
/// the source, over the slots `run` covers. A piece the assembly
/// reproduces verbatim narrows to no edit.
fn piecewise_edits<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
    run: Range<usize>,
    gap: &mut impl FnMut(usize) -> Option<&'src str>,
) -> Vec<Edit> {
    let mut edits = Vec::new();
    walk_assembly(source, blocks, rendered, order, run, gap, |span, text| {
        edits.extend(narrowed_replacement(source, span, text));
    });
    edits
}

/// Walks the pieces an assembly of `blocks` writes over the slots
/// `run` covers, handing each to `piece` as the source range it
/// replaces and the text replacing it: the block standing at each
/// destination slot, then the gap following it. A gap override
/// substitutes its own text and `None` yields the source gap verbatim.
/// The final slot of `run` contributes no gap.
fn walk_assembly<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
    run: Range<usize>,
    gap: &mut impl FnMut(usize) -> Option<&'src str>,
    mut piece: impl FnMut(TextRange, &str),
) {
    for i in run.clone() {
        let block = blocks[i];
        piece(block, &rendered[order[i]]);
        let Some(next) = (i + 1 < run.end).then(|| blocks[i + 1]) else {
            continue;
        };
        let span = TextRange::new(block.end(), next.start());
        piece(span, gap(i).unwrap_or_else(|| source.slice(span)));
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use indoc::indoc;

    use super::*;
    use crate::testing::{first_def, parse};

    #[test]
    fn assemble_blocks_mixes_overridden_and_source_gaps() {
        let source = parse("def a(): pass\ndef b(): pass\ndef c(): pass\n");
        let blocks: Vec<TextRange> = source.ast().body.iter().map(Ranged::range).collect();
        let rendered: Vec<Cow<str>> = blocks
            .iter()
            .map(|&b| Cow::Borrowed(source.slice(b)))
            .collect();
        let order = vec![0, 1, 2];
        let assembled = assemble_blocks(&source, &blocks, &rendered, &order, |i| {
            (i == 0).then_some(" ; ")
        });
        assert_eq!(assembled, "def a(): pass ; def b(): pass\ndef c(): pass");
    }

    #[test]
    fn assemble_blocks_substitutes_gap_when_override_returns_some() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let blocks: Vec<TextRange> = source.ast().body.iter().map(Ranged::range).collect();
        let rendered: Vec<Cow<str>> = blocks
            .iter()
            .map(|&b| Cow::Borrowed(source.slice(b)))
            .collect();
        let order = vec![0, 1];
        let assembled = assemble_blocks(&source, &blocks, &rendered, &order, |_| Some(" ; "));
        assert_eq!(assembled, "def a(): pass ; def b(): pass");
    }

    #[test]
    fn reorder_text_inline_swaps_two_items() {
        let source = parse("def f(b, a): pass\n");
        let func = first_def(&source);
        let params = &func.parameters;
        let (cow, _) = reorder_text(
            &source,
            &params.args,
            |p| Some(p.parameter.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_matches!(cow, Cow::Owned(_));
        assert_eq!(&*cow, "a, b");
    }

    #[test]
    fn reorder_text_pins_non_classified() {
        let source = parse(indoc! {"
            def b(): pass
            CONST = 1
            def a(): pass
        "});
        let body = &source.ast().body;
        let (cow, _) = reorder_text(
            &source,
            body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_eq!(&*cow, "def a(): pass\nCONST = 1\ndef b(): pass");
    }

    #[test]
    fn reorder_text_returns_borrowed_when_already_sorted_and_no_render_change() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let (cow, _) = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_matches!(cow, Cow::Borrowed(_));
    }

    #[test]
    fn reorder_text_returns_empty_borrowed_for_empty_items() {
        let source = parse("");
        let body = &source.ast().body;
        let (cow, _) = reorder_text(
            &source,
            body.as_slice(),
            |stmt: &ruff_python_ast::Stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Borrowed(source.slice(block)),
        );
        assert_matches!(cow, Cow::Borrowed(""));
    }

    #[test]
    fn reorder_text_returns_owned_when_render_block_owns_even_without_sort() {
        let source = parse("def a(): pass\ndef b(): pass\n");
        let (cow, _) = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |i, block| {
                let slice = source.slice(block);
                if i == 0 {
                    Cow::Owned(slice.replace("def a", "def A"))
                } else {
                    Cow::Borrowed(slice)
                }
            },
        );
        assert_matches!(cow, Cow::Owned(_));
        assert!((*cow).contains("def A"));
    }

    #[test]
    fn reorder_text_returns_owned_when_sort_and_render_owned_combine() {
        let source = parse("def b(): pass\ndef a(): pass\n");
        let (cow, _) = reorder_text(
            &source,
            &source.ast().body,
            |stmt| stmt.as_function_def_stmt().map(|f| f.name.as_str()),
            |_, block| Cow::Owned(source.slice(block).replace("def ", "DEF ")),
        );
        assert_matches!(cow, Cow::Owned(_));
        assert_eq!(&*cow, "DEF a(): pass\nDEF b(): pass");
    }
}

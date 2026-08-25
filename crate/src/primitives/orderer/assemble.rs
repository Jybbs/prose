//! Assembles reordered member blocks back into source text, the
//! separated and cell-edit forms included.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_parser::parse_module;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        edit::{any_owned, narrowed_replacement},
        range::blocks_span,
        slots::slot_runs,
        splice::{reparse_window, splice_parses},
    },
    source::Source,
};

use super::*;

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
    for (i, (&idx, block)) in order.iter().zip(blocks).enumerate() {
        out.push_str(&rendered[idx]);
        if let Some(next) = blocks.get(i + 1) {
            let gap = gap_override(i)
                .unwrap_or_else(|| source.slice(TextRange::new(block.end(), next.start())));
            out.push_str(gap);
        }
    }
    out
}

/// Assembles `rendered` in `order` with `gap`, returning the covered
/// span alongside the text. Short-circuits to a borrow of the source
/// span when no child rewrote and `order` is identity, unless `forced`
/// holds, the signal a gap override reshapes spacing without reordering.
pub(crate) fn assemble_or_borrow<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
    forced: bool,
    gap: impl FnMut(usize) -> Option<&'src str>,
) -> (Cow<'src, str>, TextRange) {
    let span = blocks_span(blocks);
    if !forced && !any_owned(rendered) && is_identity(order) {
        return (Cow::Borrowed(source.slice(span)), span);
    }
    (
        Cow::Owned(assemble_blocks(source, blocks, rendered, order, gap)),
        span,
    )
}

/// Concatenates `block_texts` in `order`, re-emitting each member's comma so
/// it lands after the value and before any trailing comment. `value_ends`
/// split the code from each comma-and-comment tail. Non-last slots carry a
/// comma, the new-last slot matches `source_last_has_comma`, and a blank line
/// follows every slot in `divider_slots`. Every break it writes takes the
/// ending `source` carries.
pub(crate) fn assemble_separated(
    source: &Source,
    value_ends: &[TextSize],
    blocks: &[TextRange],
    block_texts: &[Cow<'_, str>],
    order: &[usize],
    divider_slots: &[usize],
    source_last_has_comma: bool,
) -> String {
    let newline = source.newline_str();
    let mut out = String::with_capacity(blocks_span(blocks).len().to_usize());
    for (slot, &idx) in order.iter().enumerate() {
        let block_text = &block_texts[idx];
        let tail_len = (blocks[idx].end() - value_ends[idx]).to_usize();
        let (code, tail) = block_text.split_at(block_text.len() - tail_len);
        let (separator, comment) = tail.split_at(tail.find('#').unwrap_or(tail.len()));
        out.push_str(code);
        let is_last = slot + 1 == order.len();
        if !is_last || source_last_has_comma {
            out.push(',');
        }
        if !comment.is_empty() {
            out.extend(separator.chars().filter(|&c| c != ','));
            out.push_str(comment);
        }
        if !is_last {
            out.push_str(newline);
            if divider_slots.binary_search(&slot).is_ok() {
                out.push_str(newline);
            }
        }
    }
    out
}

/// Assembles a body rewrite into edits: one narrowed edit per notebook
/// cell the `blocks` span, or a single body-spanning edit for an ordinary
/// module. The arguments mirror [`assemble_or_borrow`]. `order` never
/// crosses a cell boundary, so each cell's slots stay a contiguous run
/// that reassembles against the cell's own block span.
pub(crate) fn assembled_cell_edits<'src>(
    source: &'src Source,
    blocks: &[TextRange],
    rendered: &[Cow<'src, str>],
    order: &[usize],
    forced: bool,
    mut gap: impl FnMut(usize) -> Option<&'src str>,
) -> Vec<Edit> {
    if !source.is_notebook() {
        let (text, span) = assemble_or_borrow(source, blocks, rendered, order, forced, gap);
        return match text {
            Cow::Borrowed(_) => Vec::new(),
            Cow::Owned(owned) => narrowed_replacement(source, span, owned)
                .into_iter()
                .collect(),
        };
    }
    let mut edits = Vec::new();
    for Range { start, end } in slot_runs(blocks, |a, b| source.same_cell(a.start(), b.start())) {
        let cell = &blocks[start..end];
        let rebased: Vec<usize> = order[start..end].iter().map(|&slot| slot - start).collect();
        let assembled = assemble_blocks(source, cell, &rendered[start..end], &rebased, |slot| {
            gap(start + slot)
        });
        edits.extend(narrowed_replacement(source, blocks_span(cell), assembled));
    }
    edits
}

/// Reorders a comma-separated group laid out one member per line, the comma
/// re-emitted per slot so each member's trailing comment travels with it. Each
/// block reaches back over the own-line comments attached above its member and
/// forward through any trailing comma and comment, so both ride with the member.
/// Declines, returning a borrow, when nothing reorders or the reassembled group
/// no longer parses.
pub(crate) fn reorder_separated<'src, 'a, T, S, F>(
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
    let text = source.text();
    let (blocks, block_texts): (Vec<TextRange>, Vec<Cow<'src, str>>) = items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let start = match items[..i].last() {
                Some(prev) => leading_attached_start(source, t.start(), prev.end()),
                None => text.line_start(t.start()),
            };
            let block = TextRange::new(start, tail_end(source, t.end()));
            (block, render_block(i, block))
        })
        .unzip();
    let span = blocks_span(&blocks);
    let mut order: Vec<usize> = (0..items.len()).collect();
    let permuted = permute_full(&mut order, items, classify);
    if !permuted && !any_owned(&block_texts) {
        return (Cow::Borrowed(source.slice(span)), span);
    }
    let value_ends: Vec<TextSize> = items.iter().map(Ranged::end).collect();
    let assembled = assemble_separated(
        source,
        &value_ends,
        &blocks,
        &block_texts,
        &order,
        &[],
        last_member_has_comma(source, items),
    );
    if assembled == source.slice(span)
        || !splice_parses(
            source,
            reparse_window(source, span),
            span,
            &assembled,
            parse_module,
        )
    {
        return (Cow::Borrowed(source.slice(span)), span);
    }
    (Cow::Owned(assembled), span)
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
    let mut order: Vec<usize> = (0..items.len()).collect();
    permute_full(&mut order, items, classify);
    assemble_or_borrow(source, &blocks, &rendered, &order, false, |_| None)
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

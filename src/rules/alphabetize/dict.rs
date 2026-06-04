//! Dict-literal reordering. Sorts a dict's items single-line entries
//! before multi-line and alphabetizes within each partition by the
//! key's source slice, folding nested reorders into each item block and
//! declining when the reassembled dict no longer parses.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{DictItem, ExprDict};
use ruff_python_parser::parse_expression;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::has_keep_marker;
use crate::{
    primitives::{
        edit::{apply_inline_edits, splice_parses},
        orderer::{assemble_blocks, block_range, blocks_span, permute_full},
        range::paren_aware_range,
    },
    source::Source,
};

/// Rewrites a dict literal's items span. Returns `Some((span, text))`
/// when reordering, partition, or any nested reorder folded from `edits`
/// produces text different from the source slice. Returns `None` for
/// empty dicts, dicts marked `# prose: keep`, single-item dicts, and any
/// already-canonical case. `edits` are the leaf edits collected from the
/// dict's descendants, folded into each item block.
pub(super) fn rewrite_dict_text<'src>(
    source: &'src Source,
    d: &ExprDict,
    edits: &[Edit],
) -> Option<(TextRange, Cow<'src, str>)> {
    if d.is_empty() || has_keep_marker(source, d) {
        return None;
    }
    let [first, .., last] = d.items.as_slice() else {
        return None;
    };
    let multi_line = source.contains_line_break(first.range().cover(last.range()));
    // Widen each item to its value's paren-aware end, so a parenthesized
    // value keeps its closing parens inside the block rather than shedding
    // them into the separator tail.
    let item_ranges: Vec<TextRange> = d
        .items
        .iter()
        .map(|item| TextRange::new(item.start(), item_value_end(source, d, item)))
        .collect();
    let blocks: Vec<TextRange> = if multi_line {
        (0..d.len())
            .map(|i| block_range(source, &item_ranges, i, d.range()))
            .collect()
    } else {
        item_ranges.clone()
    };
    let span = blocks_span(&blocks);
    let block_texts: Vec<Cow<'src, str>> = blocks
        .iter()
        .map(|&block| apply_inline_edits(source, block, edits))
        .collect();
    let any_nested_rewrite = block_texts.iter().any(|c| matches!(c, Cow::Owned(_)));
    let mut order: Vec<usize> = (0..d.len()).collect();
    let permuted = permute_full(&mut order, &d.items, |item| dict_sort_key(source, item));
    let assembled = if multi_line {
        let divider_slots = partition_divider_slots(source, &order, &d.items);
        let source_last_has_comma = source.trailing_comma(d.range()).is_some();
        let value_ends: Vec<TextSize> = item_ranges.iter().map(Ranged::end).collect();
        assemble_dict_items_multiline(
            &value_ends,
            &blocks,
            &block_texts,
            &order,
            &divider_slots,
            source_last_has_comma,
        )
    } else {
        assemble_blocks(source, &blocks, &block_texts, &order, |_| None)
    };
    if !permuted && !any_nested_rewrite && assembled == source.slice(span) {
        return None;
    }
    // Decline the reorder when the reassembled dict no longer parses, the
    // safety net for irregular layouts (entries sharing a line, comments
    // inside a `**`-spread's parentheses) the block model cannot shuffle
    // cleanly.
    if !splice_parses(source, d.range(), span, &assembled, parse_expression) {
        return None;
    }
    Some((span, Cow::Owned(assembled)))
}

/// Concatenates dict-item block texts in `order`, placing each slot's
/// separator comma against the entry's value span so it lands after the
/// value and before any trailing line comment. `value_ends` carry each
/// value's paren-aware end, splitting code from the separator tail past
/// any closing parens. Non-last slots always carry a comma and the
/// new-last slot matches `source_last_has_comma`. Inserts a blank line at
/// every slot listed in `divider_slots`.
fn assemble_dict_items_multiline(
    value_ends: &[TextSize],
    blocks: &[TextRange],
    block_texts: &[Cow<'_, str>],
    order: &[usize],
    divider_slots: &[usize],
    source_last_has_comma: bool,
) -> String {
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
            out.push('\n');
            if divider_slots.binary_search(&slot).is_ok() {
                out.push('\n');
            }
        }
    }
    out
}

/// Composite dict-item sort key. `**unpacked` items return `None` and
/// pin in source position. Keyed items sort single-line entries before
/// multi-line entries and alphabetize within each partition by the
/// key's source slice.
fn dict_sort_key<'a>(source: &'a Source, item: &'a DictItem) -> Option<(u8, &'a str)> {
    let key = item.key.as_ref()?;
    let group = u8::from(source.contains_line_break(item.range()));
    Some((group, source.slice(key)))
}

/// The end offset of a dict item's value, widened past any parentheses
/// enclosing it. A multiline reorder splits each entry at this offset, so
/// excluding the closing parens would shed them into the separator tail.
fn item_value_end(source: &Source, dict: &ExprDict, item: &DictItem) -> TextSize {
    paren_aware_range((&item.value).into(), dict.into(), source.tokens()).end()
}

/// Returns the new-order slot indices after which a blank-line
/// divider should sit. A divider goes on either side of every keyed
/// multi-line entry, isolating it from its neighbors so each
/// multi-line entry forms its own alignment group downstream.
fn partition_divider_slots(source: &Source, order: &[usize], items: &[DictItem]) -> Vec<usize> {
    let is_multiline =
        |i: usize| items[i].key.is_some() && source.contains_line_break(items[i].range());
    order
        .windows(2)
        .enumerate()
        .filter(|(_, w)| is_multiline(w[0]) || is_multiline(w[1]))
        .map(|(i, _)| i)
        .collect()
}

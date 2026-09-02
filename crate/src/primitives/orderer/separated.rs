//! Assembles a reordered comma-separated group, the comma and the
//! trailing comment re-emitted per member so both travel with it.

use std::borrow::Cow;

use ruff_python_parser::parse_module;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::*;
use crate::{
    primitives::{
        edit::any_owned,
        range::blocks_span,
        splice::{reparse_window, splice_parses},
    },
    source::Source,
};

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
        let split = block_text
            .len()
            .checked_sub(tail_len)
            .expect("a rendered block keeps its comma tail");
        let (code, tail) = block_text.split_at(split);
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

/// Reorders a comma-separated group laid out one member per line, the comma
/// re-emitted per slot so each member's trailing comment travels with it. Each
/// block reaches back over the own-line comments attached above its member and
/// forward through any trailing comma and comment, so both move with the member.
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
    if items.is_empty() {
        return (Cow::Borrowed(""), TextRange::default());
    }
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

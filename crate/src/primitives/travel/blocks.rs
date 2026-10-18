//! The source extent a moved block covers and the rows frozen inside it.

use std::borrow::Cow;

use super::*;

/// The move `block`'s continuation rows make when it lands per
/// `landing`, its own start at `start` and its frozen rows flagged in
/// `frozen`. `None` where every continuation row is blank or frozen.
/// A block whose movable rows sit at or left of its item's column
/// rebases through [`hanging_travel`] or onto the landing indent,
/// whereas one whose rows sit right of that column shifts whole by
/// however far its own start moved. Rows exactly one indent step past
/// the opening row hang from it and re-seat one step past the landing,
/// unless a token of the block on that row starts at their column.
pub(crate) fn block_shift(
    source: &Source,
    block: &str,
    frozen: &[bool],
    start: TextSize,
    landing: Landing,
) -> Option<Travel> {
    let floor = movable_floor(block, frozen)?;
    let item = source.column_of(landing.item);
    if follows(
        floor,
        source.line_indent_width(start),
        item,
        Head::of(source, start),
    ) {
        return Some(Travel::rigid(
            landing.column.cast_signed() - source.column_of(start).cast_signed(),
        ));
    }
    let rebase = if floor > item {
        item_indent(landing.indent)
    } else {
        landing.indent
    };
    Some(
        hanging_travel(block, frozen, landing)
            .unwrap_or_else(|| Travel::rigid(rebase.cast_signed() - floor.cast_signed())),
    )
}

/// Returns the line range of each movable continuation row of the value
/// at `range`, its start opening inside an item at column `item`, that
/// moves with that start. Every row moves where the rows move rigidly
/// the way [`block_shift`] moves them, per [`follows`], and otherwise
/// each row moves only where it sits beneath a token of the value, per
/// [`rows_beneath`]. A value whose first row leaves a bracket open hangs
/// from that row instead wherever its interior rows sit no further in
/// than one indent step past it, per [`hanging_travel`]. A row a
/// row-spanning string freezes, a row indented with a tab, and a blank
/// row stay where the source wrote them.
pub(crate) fn carried_rows(source: &Source, range: TextRange, item: usize) -> Vec<TextRange> {
    if !source.contains_line_break(range) {
        return Vec::new();
    }
    let block = source.slice(range);
    let frozen = frozen_rows(source, range);
    let indent = source.line_indent_width(range.start());
    let landing = Landing::own_row(TextSize::default(), indent);
    let head = Head::of(source, range.start());
    if hanging_travel(block, &frozen, landing).is_some_and(|travel| travel.rows >= 0)
        || movable_floor(block, &frozen).is_none_or(|floor| !follows(floor, indent, item, head))
    {
        return rows_beneath(source, range, &frozen);
    }
    movable_rows(block, &frozen)
        .filter(|line| !leading_indentation(line.as_str()).contains('\t'))
        .map(|line| line.range() + range.start())
        .collect()
}

/// One flag per row of the block at `range`, set for every row opening
/// strictly inside a string token that itself spans rows, the rows a
/// move holds.
pub(crate) fn frozen_rows(source: &Source, range: TextRange) -> Vec<bool> {
    let head = source.line_index(range.start()).get();
    let mut frozen = vec![false; source.line_index(range.end()).get() - head + 1];
    for token in source.tokens_overlapping(range) {
        if !matches!(
            token.kind(),
            TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle
        ) || !source.contains_line_break(token.range())
        {
            continue;
        }
        let opens = source.line_index(token.start()).get();
        let closes = source.line_index(token.end()).get();
        let hi = (closes + 1).saturating_sub(head).min(frozen.len());
        let lo = (opens + 1).saturating_sub(head).min(hi);
        frozen[lo..hi].fill(true);
    }
    frozen
}

/// `range`'s text read through `edits`, hung from a row sitting at
/// `indent`, so every movable continuation row lands one step further
/// in and a closing row of the block's own returns to `indent`. A block
/// whose head leaves a bracket open takes that bracket's shape through
/// [`hanging_travel`], and one that does not moves rigidly.
pub(crate) fn hung_block_through<'s>(
    source: &'s Source,
    range: TextRange,
    edits: &[Edit],
    indent: usize,
) -> Cow<'s, str> {
    let block = apply_inline_edits(source, range, edits);
    let mut frozen = frozen_rows(source, range);
    let head = source.line_index(range.start()).get();
    for edit in edits
        .iter()
        .rev()
        .filter(|edit| range.contains_range(edit.range()))
    {
        if edit.content().is_some_and(spans_rows) {
            return block;
        }
        let first = source.line_index(edit.start()).get() - head;
        let last = source.line_index(edit.end()).get() - head;
        if last > first {
            let merged = frozen.drain(first + 1..=last).any(|held| held);
            frozen[first] |= merged;
        }
    }
    let landing = Landing::own_row(range.start(), indent);
    let opens_a_bracket = block
        .universal_newlines()
        .next()
        .is_some_and(|head| ends_on_an_opener(row_tokens(head.as_str())));
    let travel = if opens_a_bracket {
        // A block whose shape [`hanging_travel`] cannot follow holds
        // where it sits.
        match hanging_travel(&block, &frozen, landing) {
            Some(travel) => travel,
            None => return block,
        }
    } else {
        let Some(floor) = movable_floor(&block, &frozen) else {
            return block;
        };
        Travel::rigid(item_indent(indent).cast_signed() - floor.cast_signed())
    };
    if travel.is_still() {
        return block;
    }
    Cow::Owned(shifted_rows(&block, travel, &frozen))
}

/// `range`'s source text placed per `landing`, its continuation rows
/// traveling and every row a row-spanning string part freezes left
/// where the source wrote it. Borrowed where the block holds no movable
/// continuation row or already sits where it lands.
pub(crate) fn placed_block(source: &Source, range: TextRange, landing: Landing) -> Cow<'_, str> {
    let block = source.slice(range);
    let frozen = frozen_rows(source, range);
    match block_shift(source, block, &frozen, range.start(), landing) {
        Some(travel) if !travel.is_still() => Cow::Owned(shifted_rows(block, travel, &frozen)),
        _ => Cow::Borrowed(block),
    }
}

/// `block`'s continuation rows moved per `travel`, every blank row
/// passing through as written and the block borrowed where no row
/// moves. A caller screens the block through [`spans_a_string_part`]
/// first.
pub(crate) fn shifted_block(block: &str, travel: Travel) -> Cow<'_, str> {
    if travel.is_still() {
        return Cow::Borrowed(block);
    }
    Cow::Owned(shifted_rows(block, travel, &[]))
}

/// True where a string part inside `expr` itself spans rows. A stacked
/// run of single-line parts carries its break between parts and reads
/// false.
pub(crate) fn spans_a_string_part(source: &Source, expr: &Expr) -> bool {
    any_over_expr(expr, |e| {
        StringLike::try_from(e)
            .is_ok_and(|run| run.parts().any(|part| source.contains_line_break(part)))
    })
}

/// True where a block moves rigidly with its start. The block's first row
/// sits at `indent` inside an item at column `item`, and `floor` is the
/// column its leftmost movable row starts at. The block moves where the
/// floor sits past the item's column and either lies off the column one
/// indent step past the first row, where a hanging row would sit, or
/// starts beneath a token of `head`, the value's opening row.
fn follows(floor: usize, indent: usize, item: usize, head: Head) -> bool {
    floor > item && (floor != item_indent(indent) || head.anchors(floor))
}

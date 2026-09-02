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
/// the opening row hang from it and re-seat one step past the landing.
pub(crate) fn block_shift(
    source: &Source,
    block: &str,
    frozen: &[bool],
    start: TextSize,
    landing: Landing,
) -> Option<Travel> {
    let floor = movable_floor(block, frozen)?;
    let past_item = floor > source.column_of(landing.item);
    if past_item && floor != item_indent(source.line_indent_width(start)) {
        return Some(Travel::rigid(
            landing.column.cast_signed() - source.column_of(start).cast_signed(),
        ));
    }
    let rebase = if past_item {
        item_indent(landing.indent)
    } else {
        landing.indent
    };
    Some(
        hanging_travel(block, frozen, landing)
            .unwrap_or_else(|| Travel::rigid(rebase.cast_signed() - floor.cast_signed())),
    )
}

/// One flag per row of the block at `range`, set for every row opening
/// strictly inside a string token that itself spans rows. Shifting such
/// a row would pad the string's own interior, so a move holds it.
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
    let frozen = frozen_rows(source, range);
    let landing = Landing::own_row(range.start(), indent);
    let opens_a_bracket = block
        .universal_newlines()
        .next()
        .is_some_and(|head| head.trim_end().ends_with(OPENERS));
    let travel = if opens_a_bracket {
        // The bracket the head leaves open lays out the rows beneath it,
        // so a block whose shape [`hanging_travel`] cannot follow holds
        // where it sits rather than hanging from the row it lands on.
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
/// travelling and every row a row-spanning string part freezes left
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
/// first, whose interior a move would pad.
pub(crate) fn shifted_block(block: &str, travel: Travel) -> Cow<'_, str> {
    if travel.is_still() {
        return Cow::Borrowed(block);
    }
    Cow::Owned(shifted_rows(block, travel, &[]))
}

/// True where a string part inside `expr` itself spans rows, whose
/// interior a re-indent would pad. A stacked run of single-line parts
/// carries its break between parts and moves whole, so it reads false.
pub(crate) fn spans_a_string_part(source: &Source, expr: &Expr) -> bool {
    any_over_expr(expr, |e| {
        StringLike::try_from(e)
            .is_ok_and(|run| run.parts().any(|part| source.contains_line_break(part)))
    })
}

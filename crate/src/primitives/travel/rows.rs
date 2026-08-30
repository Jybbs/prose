//! The rows a block may move and the floor that movement stops at.

use super::*;

/// The move for a block whose first row leaves a bracket open, seating
/// the shallowest interior row one `INDENT_STEP` inside `landing.indent`
/// and a closing row of the block's own back on that indent, the shape
/// `explode_parens` writes. A closing row the row move already lands
/// there travels with the rest. `None` where the first row opens no
/// bracket, where a row-spanning string opens on it, or where an
/// interior row itself opens with a closing bracket, whose depth one
/// move cannot follow, a last row opening with the closer of a bracket
/// an interior row opened reading as interior too.
pub(super) fn hanging_travel(block: &str, frozen: &[bool], landing: Landing) -> Option<Travel> {
    let head = block.universal_newlines().next()?;
    if !head.trim_end().ends_with(OPENERS) || frozen.get(1) == Some(&true) {
        return None;
    }
    let rows: Vec<Line> = movable_rows(block, frozen).collect();
    let opens_with_closer = |line: &Line| line.trim_start().starts_with(CLOSERS);
    let close = rows
        .last()
        .filter(|line| opens_with_closer(line) && closes_the_head(line.trim_start()))
        .map(|line| indent_width(line));
    let interior = &rows[..rows.len() - usize::from(close.is_some())];
    if interior.iter().any(opens_with_closer) {
        return None;
    }
    let floor = interior.iter().map(|line| indent_width(line)).min()?;
    let rows = item_indent(landing.indent).cast_signed() - floor.cast_signed();
    Some(Travel {
        rows,
        closer: close
            .filter(|indent| indent.saturating_add_signed(rows) != landing.indent)
            .map(|_| landing.indent),
    })
}

/// The least indent among the movable non-blank continuation rows of
/// `block`, `None` where every continuation row is blank or frozen. An
/// empty `frozen` holds no row, the shape a caller passes when nothing
/// inside the block spans rows.
pub(super) fn movable_floor(block: &str, frozen: &[bool]) -> Option<usize> {
    movable_rows(block, frozen)
        .map(|line| indent_width(&line))
        .min()
}

/// `block`'s continuation rows moved per `travel`, each blank row and
/// each row `frozen` marks passing through as written.
pub(super) fn shifted_rows(block: &str, travel: Travel, frozen: &[bool]) -> String {
    let last = block
        .split_inclusive('\n')
        .enumerate()
        .filter(|(row, line)| is_movable(*row, line, frozen))
        .map(|(row, _)| row)
        .last();
    let mut out = String::with_capacity(block.len());
    for (row, line) in block.split_inclusive('\n').enumerate() {
        if !is_movable(row, line, frozen) {
            out.push_str(line);
            continue;
        }
        let placed = travel.placed(indent_width(line), Some(row) == last);
        out.push_str(&" ".repeat(placed));
        out.push_str(&line[leading_indentation(line).len()..]);
    }
    out
}

/// True where the closer `row` opens with closes the bracket the block's
/// head left open rather than one an interior row opened, meaning no
/// closer later on the row is left unmatched by an opener ahead of it.
fn closes_the_head(row: &str) -> bool {
    let mut lexer = lex(row, Mode::Expression);
    let mut depth = 0_usize;
    // The leading closer is the one under test.
    lexer.next_token();
    loop {
        let kind = lexer.next_token();
        if kind == TokenKind::EndOfFile {
            return true;
        }
        if is_opener(kind) {
            depth += 1;
        } else if is_closer(kind) {
            let Some(shallower) = depth.checked_sub(1) else {
                return false;
            };
            depth = shallower;
        }
    }
}

/// True for a non-blank continuation row at `row` that `frozen` leaves
/// free to move.
fn is_movable(row: usize, line: &str, frozen: &[bool]) -> bool {
    row > 0 && frozen.get(row) != Some(&true) && !line.trim().is_empty()
}

/// Yields each movable non-blank continuation row of `block`, skipping
/// the rows `frozen` marks.
fn movable_rows<'b>(block: &'b str, frozen: &'b [bool]) -> impl Iterator<Item = Line<'b>> {
    block
        .universal_newlines()
        .enumerate()
        .filter(|(row, line)| is_movable(*row, line, frozen))
        .map(|(_, line)| line)
}

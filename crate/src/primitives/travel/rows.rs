//! The rows a block may move, the floor that movement stops at, and the
//! seat each continuation row takes against the bracket it sits inside.

use super::*;

/// What a continuation row sits against inside the innermost bracket
/// open ahead of it, per [`seat`].
pub(crate) enum Seat<'t> {
    /// The row sits on this token's column.
    Token(&'t Token),
    /// The row hangs from the row the bracket opens on.
    Hangs(OneIndexed),
    /// The row aligns to nothing.
    Free,
}

/// The move for a block whose first row leaves a bracket open, seating
/// the shallowest interior row one `INDENT_STEP` inside `landing.indent`
/// and a closing row of the block's own back on that indent. A closing
/// row the row move already lands there travels with the rest. `None`
/// where the first row opens no bracket, where a row-spanning string
/// opens on it, or where an interior row itself opens with a closing
/// bracket, a last row opening with the closer of a bracket an interior
/// row opened reading as interior too.
pub(super) fn hanging_travel(block: &str, frozen: &[bool], landing: Landing) -> Option<Travel> {
    let head = block.universal_newlines().next()?;
    if !ends_on_an_opener(row_tokens(head.as_str())) || frozen.get(1) == Some(&true) {
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

/// Returns the line range of each movable continuation row of the value
/// at `range` that sits, per [`seat`], on a token the value writes on its
/// own opening row or on a token of a row that itself moves, or that
/// hangs from a row that moves. The value moves along its opening row
/// while that row's indent stays, so a row hanging from the opening row
/// stays where it was written, as does a tab-indented row and every
/// row `frozen` holds.
pub(super) fn rows_beneath(source: &Source, range: TextRange, frozen: &[bool]) -> Vec<TextRange> {
    let block = source.slice(range);
    let opening = source.line_index(range.start());
    let tokens: Vec<&Token> = tokens_within(
        source,
        TextRange::new(source.text().line_start(range.start()), range.end()),
    )
    .filter(|token| !token.kind().is_trivia())
    .collect();
    // Every row that moves sits on a token of the opening row or of a row
    // that moves, so the first to move sits on a token the value writes
    // past a bracket on the opening row, and no row moves where none does.
    let opening_end = source.text().line_end(range.start());
    let seats: Vec<usize> = tokens
        .iter()
        .take_while(|token| token.start() < opening_end)
        .skip_while(|token| !is_opener(token.kind()))
        .skip(1)
        .filter(|token| token.start() >= range.start())
        .map(|token| source.column_of(token.start()))
        .collect();
    if !movable_rows(block, frozen).any(|line| seats.contains(&indent_width(&line))) {
        return Vec::new();
    }
    let mut moving: FxHashSet<OneIndexed> = FxHashSet::default();
    let mut rows = Vec::new();
    for line in movable_rows(block, frozen) {
        if leading_indentation(line.as_str()).contains('\t') {
            continue;
        }
        let start = range.start() + line.start();
        let moves = match seat(source, &tokens, |_| true, start, indent_width(&line)) {
            Some(Seat::Token(token)) => {
                let row = source.line_index(token.start());
                if row == opening {
                    token.start() >= range.start()
                } else {
                    moving.contains(&row)
                }
            }
            Some(Seat::Hangs(row)) => moving.contains(&row),
            Some(Seat::Free) | None => false,
        };
        if moves {
            moving.insert(source.line_index(start));
            rows.push(line.range() + range.start());
        }
    }
    rows
}

/// Returns the seat of the continuation row opening at `start` at
/// `indent`, read against the innermost bracket `tokens` leave open
/// ahead of it.
/// Only a token `anchors` admits counts, and only one past the bracket
/// on the bracket's own row. The row sits on the first such token where it
/// opens at that token's column, and otherwise on a row-anchor token at
/// its column. It hangs where nothing follows the bracket on its row or
/// where it sits one or two indent steps past that row, and otherwise
/// sits on any token at its column. `None` where no bracket is open
/// ahead of the row.
pub(crate) fn seat<'t>(
    source: &Source,
    tokens: &[&'t Token],
    anchors: impl Fn(&Token) -> bool,
    start: TextSize,
    indent: usize,
) -> Option<Seat<'t>> {
    let opener = open_brackets(
        tokens
            .iter()
            .copied()
            .take_while(|token| token.start() < start),
    )
    .pop()?;
    let opener_row = source.line_index(opener);
    let opener_end = source.text().line_end(opener);
    let after: Vec<&Token> = tokens[tokens.partition_point(|token| token.start() <= opener)..]
        .iter()
        .copied()
        .take_while(|token| token.start() < opener_end)
        .filter(|token| anchors(token))
        .collect();
    let at = |token: &Token| source.column_of(token.start()) == indent;
    let row_indent = source.line_indent_width(opener);
    Some(match after.first() {
        None => Seat::Hangs(opener_row),
        Some(first) if at(first) => Seat::Token(first),
        Some(_) => {
            if let Some(code) = after
                .iter()
                .find(|token| at(token) && is_row_anchor(token.kind()))
            {
                Seat::Token(code)
            } else if indent == row_indent + INDENT_STEP || indent == row_indent + 2 * INDENT_STEP {
                Seat::Hangs(opener_row)
            } else if let Some(token) = after.iter().find(|token| at(token)) {
                Seat::Token(token)
            } else {
                Seat::Free
            }
        }
    })
}

/// `block`'s continuation rows moved per `travel`, each blank row and
/// each row `frozen` marks passing through as written.
pub(super) fn shifted_rows(block: &str, travel: Travel, frozen: &[bool]) -> String {
    let rows: Vec<Line> = block.universal_newlines().collect();
    let last = rows
        .iter()
        .enumerate()
        .rfind(|(row, line)| is_movable(*row, line.as_str(), frozen))
        .map(|(row, _)| row);
    let mut out = String::with_capacity(block.len());
    for (row, line) in rows.iter().enumerate() {
        if !is_movable(row, line.as_str(), frozen) {
            out.push_str(line.as_full_str());
            continue;
        }
        let placed = travel.placed(indent_width(line.as_str()), Some(row) == last);
        out.extend(std::iter::repeat_n(' ', placed));
        out.push_str(line.as_full_str().trim_whitespace_start());
    }
    out
}

/// True where the closer `row` opens with closes the bracket the block's
/// head left open rather than one an interior row opened, meaning no
/// closer later on the row is left unmatched by an opener ahead of it.
fn closes_the_head(row: &str) -> bool {
    // The leading closer is the one under test.
    row_tokens(row)
        .skip(1)
        .try_fold(0_usize, |depth, (kind, _)| {
            if is_opener(kind) {
                Some(depth + 1)
            } else if is_closer(kind) {
                depth.checked_sub(1)
            } else {
                Some(depth)
            }
        })
        .is_some()
}

/// True for a non-blank continuation row at `row` that `frozen` leaves
/// free to move.
pub(crate) fn is_movable(row: usize, line: &str, frozen: &[bool]) -> bool {
    row > 0 && frozen.get(row) != Some(&true) && !line.trim().is_empty()
}

/// Yields each movable non-blank continuation row of `block`, skipping
/// the rows `frozen` marks.
pub(super) fn movable_rows<'b>(
    block: &'b str,
    frozen: &'b [bool],
) -> impl Iterator<Item = Line<'b>> {
    block
        .universal_newlines()
        .enumerate()
        .filter(|(row, line)| is_movable(*row, line, frozen))
        .map(|(_, line)| line)
}

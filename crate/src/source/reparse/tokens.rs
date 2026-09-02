//! Merges each reparsed window's tokens into the stream a source holds.

use ruff_python_ast::{
    Stmt,
    token::{Token, TokenFlags, TokenKind, Tokens},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{deltas::Deltas, flags::retargeted, window::is_code};

/// One window's held span and the span the woven text holds it at,
/// whether it is a window of the module body, which any count of
/// statements may fill, the statements its reparse produced, the
/// tokens of that reparse the merge reads, and the levels its end
/// moved.
pub(super) struct Reparsed {
    /// The levels the window's end moved past its start, which the
    /// `Dedent` run past the window closes in addition.
    pub(super) delta: isize,
    pub(super) fresh: Vec<Token>,
    pub(super) held: TextRange,
    pub(super) run: bool,
    pub(super) slid: TextRange,
    pub(super) stmts: Vec<Stmt>,
}

/// The `Dedent` run the merge holds back until the code token it
/// closes blocks ahead of, and the levels the window behind it moved
/// its end, which that run closes in addition.
#[derive(Default)]
struct Dedents {
    held: Vec<Token>,
    owed: isize,
}

impl Dedents {
    /// Appends the run at `at`, its count moved by the levels owed,
    /// and clears both. A window's end never drops below its start and
    /// the next code token never sits deeper than that start, so the
    /// count never goes negative.
    fn seat(&mut self, merged: &mut Vec<Token>, at: TextSize, held_text: &str) {
        let count = self.held.len().cast_signed() + self.owed;
        debug_assert!(
            count >= 0,
            "a window closes no fewer levels than its start opened"
        );
        let template = self.held.first().copied();
        merged.extend((0..count.max(0)).map(|_| {
            template.map_or_else(
                || Token::new(TokenKind::Dedent, TextRange::empty(at), TokenFlags::empty()),
                |dedent| retargeted(dedent, held_text, TextRange::empty(at)),
            )
        }));
        self.held.clear();
        self.owed = 0;
    }
}

/// `held` with every token inside a window dropped, every token outside
/// one slid past the edits, and each window's own tokens merged in.
///
/// A window's reparse closes on zero-width tokens at its own end, where
/// the held stream carries the real ones, so the merge takes only those
/// opening before it, and opens on none of the `Dedent` run the held
/// stream carries at its start. Every held `Dedent` run is seated where
/// the lexer emits it, ahead of the next code token or at `len`, the
/// woven text's end, its count moved by the levels the window ahead of
/// it moved its end. `held_text` is the buffer `held`'s own ranges
/// index, read to rebuild the flags of each token the slide moves.
pub(super) fn spliced(
    held: &Tokens,
    held_text: &str,
    len: TextSize,
    deltas: &Deltas,
    windows: &[Reparsed],
) -> Tokens {
    let fresh: usize = windows.iter().map(|window| window.fresh.len()).sum();
    let mut merged = Vec::with_capacity(held.len() + fresh);
    let (still, moving) = windows.first().map_or((&held[..], &[][..]), |window| {
        held.split_at(window.held.start())
    });
    merged.extend_from_slice(still);
    let mut next = 0;
    let mut dedents = Dedents::default();
    for token in moving {
        while let Some(window) = windows
            .get(next)
            .filter(|window| window.held.end() <= token.start())
        {
            merge_window(&mut merged, window, &mut dedents, held_text);
            next += 1;
        }
        if windows
            .get(next)
            .is_some_and(|window| reparsed(window.held, token.range()))
        {
            continue;
        }
        if token.kind() == TokenKind::Dedent {
            dedents.held.push(*token);
            continue;
        }
        let slid = deltas.slide_token(token.range());
        if is_code(token.kind()) {
            dedents.seat(&mut merged, slid.start(), held_text);
        }
        merged.push(if slid == token.range() {
            *token
        } else {
            retargeted(*token, held_text, slid)
        });
    }
    for window in &windows[next..] {
        merge_window(&mut merged, window, &mut dedents, held_text);
    }
    dedents.seat(&mut merged, len, held_text);
    debug_assert!(
        merged.is_sorted_by_key(Ranged::start),
        "the merged token stream ascends, as its binary searches read it",
    );
    Tokens::new(merged)
}

/// Appends `window`'s fresh tokens, seating the held `Dedent` run ahead
/// of the first code token the window holds, where the lexer emits it
/// once the trivia above that token has passed, or at the window's end
/// where it holds none, and leaves the levels the window's end moved
/// owed to the run past it.
fn merge_window(
    merged: &mut Vec<Token>,
    window: &Reparsed,
    dedents: &mut Dedents,
    held_text: &str,
) {
    let at = window
        .fresh
        .iter()
        .find(|token| is_code(token.kind()))
        .map_or(window.slid.end(), Ranged::start);
    let split = window.fresh.partition_point(|token| token.start() < at);
    merged.extend_from_slice(&window.fresh[..split]);
    dedents.seat(merged, at, held_text);
    merged.extend_from_slice(&window.fresh[split..]);
    dedents.owed = window.delta;
}

/// True where a token over `range` is one `window`'s own reparse
/// produces again. A zero-width token at the window's start is not,
/// being the `Dedent` run closing a block the reparse never opened.
fn reparsed(window: TextRange, range: TextRange) -> bool {
    window.contains(range.start()) && !(range.is_empty() && range.start() == window.start())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::range;

    #[rstest]
    #[case::ahead_of_the_window(range(2, 4), false)]
    #[case::opening_the_window(range(10, 14), true)]
    #[case::inside_the_window(range(12, 16), true)]
    #[case::closing_the_window(range(18, 20), true)]
    #[case::at_the_window_end(range(20, 22), false)]
    #[case::past_the_window(range(24, 26), false)]
    #[case::zero_width_at_the_window_start(range(10, 10), false)]
    #[case::zero_width_inside_the_window(range(15, 15), true)]
    #[case::zero_width_at_the_window_end(range(20, 20), false)]
    fn reparsed_reads_which_tokens_a_window_produces_again(
        #[case] token: TextRange,
        #[case] expected: bool,
    ) {
        assert_eq!(reparsed(range(10, 20), token), expected);
    }
}

//! Merges each reparsed window's tokens into the stream a source holds.

use ruff_python_ast::{
    Stmt,
    token::{Token, TokenKind, Tokens},
};
use ruff_text_size::{Ranged, TextRange};

use super::{deltas::Deltas, flags::retargeted, window::is_code};

/// One window's held span and the span the woven text holds it at,
/// whether it is a window of the module body, which any count of
/// statements may fill, the statements its reparse produced, and the
/// tokens of that reparse the merge reads.
pub(super) struct Reparsed {
    pub(super) fresh: Vec<Token>,
    pub(super) held: TextRange,
    pub(super) run: bool,
    pub(super) slid: TextRange,
    pub(super) stmts: Vec<Stmt>,
}

/// `held` with every token inside a window dropped, every token outside
/// one slid past the edits, and each window's own tokens merged in.
///
/// A window's reparse closes on zero-width tokens at its own end, where
/// the held stream carries the real ones, so the merge takes only those
/// opening before it, and opens on none of the `Dedent` run the held
/// stream carries at its start, which the merge seats where the lexer
/// emits it. `held_text` is the buffer `held`'s own ranges index, read
/// to rebuild the flags of each token the slide moves.
pub(super) fn spliced(
    held: &Tokens,
    held_text: &str,
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
    let mut dedents: Vec<Token> = Vec::new();
    for token in moving {
        while let Some(window) = windows
            .get(next)
            .filter(|window| window.held.end() <= token.start())
        {
            merge_window(&mut merged, window, &mut dedents, held_text);
            next += 1;
        }
        if let Some(window) = windows.get(next) {
            if reparsed(window.held, token.range()) {
                continue;
            }
            if token.kind() == TokenKind::Dedent && token.start() == window.held.start() {
                dedents.push(*token);
                continue;
            }
        }
        let slid = deltas.slide_token(token.range());
        merged.push(if slid == token.range() {
            *token
        } else {
            retargeted(*token, held_text, slid)
        });
    }
    for window in &windows[next..] {
        merge_window(&mut merged, window, &mut dedents, held_text);
    }
    debug_assert!(
        merged.is_sorted_by_key(Ranged::start),
        "the merged token stream ascends, as its binary searches read it",
    );
    Tokens::new(merged)
}

/// Appends `window`'s fresh tokens, seating the `Dedent` run the held
/// stream carried at the window's start ahead of the first code token
/// the window holds, where the lexer emits it once the trivia above
/// that token has passed, or at the window's end where it holds none.
fn merge_window(
    merged: &mut Vec<Token>,
    window: &Reparsed,
    dedents: &mut Vec<Token>,
    held_text: &str,
) {
    let at = window
        .fresh
        .iter()
        .find(|token| is_code(token.kind()))
        .map_or(window.slid.end(), Ranged::start);
    let split = window.fresh.partition_point(|token| token.start() < at);
    merged.extend_from_slice(&window.fresh[..split]);
    merged.extend(
        dedents
            .drain(..)
            .map(|dedent| retargeted(dedent, held_text, TextRange::empty(at))),
    );
    merged.extend_from_slice(&window.fresh[split..]);
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

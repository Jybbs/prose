//! Merges each reparsed window's tokens into the stream a source holds.

use ruff_python_ast::{
    Stmt,
    token::{Token, Tokens},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{deltas::Deltas, flags::retargeted};

/// One window's held span and the statement and tokens its reparse
/// produced, the statement's own range being where it landed.
pub(super) struct Reparsed {
    pub(super) fresh: Tokens,
    pub(super) held: TextRange,
    pub(super) stmt: Stmt,
}

/// `held` with every token inside a window dropped, every token outside
/// one slid past the edits, and each window's own tokens merged in.
///
/// A window's reparse closes on zero-width tokens at its own end, where
/// the held stream carries the real ones, so the merge takes only those
/// opening before it. `held_text` is the buffer `held`'s own ranges
/// index, read to rebuild the flags of each token the slide moves.
pub(super) fn spliced(
    held: &Tokens,
    held_text: &str,
    deltas: &Deltas,
    windows: &[Reparsed],
) -> Tokens {
    let fresh: usize = windows.iter().map(|window| window.fresh.len()).sum();
    let mut merged = Vec::with_capacity(held.len() + fresh);
    let still = windows.first().map_or(held.len(), |window| {
        held.partition_point(|token| token.end() <= window.held.start())
    });
    merged.extend_from_slice(&held[..still]);
    let mut next = 0;
    for token in &held[still..] {
        while let Some(window) = windows
            .get(next)
            .filter(|window| window.held.end() <= token.start())
        {
            merged.extend(opening_before(&window.fresh, window.stmt.end()));
            next += 1;
        }
        if windows
            .get(next)
            .is_some_and(|window| reparsed(window.held, token.range()))
        {
            continue;
        }
        let slid = deltas.slide(token.range());
        merged.push(if slid == token.range() {
            *token
        } else {
            retargeted(*token, held_text, slid)
        });
    }
    for window in &windows[next..] {
        merged.extend(opening_before(&window.fresh, window.stmt.end()));
    }
    debug_assert!(
        merged
            .windows(2)
            .all(|pair| pair[0].start() <= pair[1].start()),
        "the merged token stream ascends, as its binary searches read it",
    );
    Tokens::new(merged)
}

/// The tokens of `fresh` that open before `end`.
fn opening_before(fresh: &Tokens, end: TextSize) -> impl Iterator<Item = Token> + use<'_> {
    fresh
        .iter()
        .take_while(move |token| token.start() < end)
        .copied()
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

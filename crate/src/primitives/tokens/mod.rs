//! Token-kind predicates over the bracket delimiters and the
//! interpolated-string openers, the characters those delimiters are
//! written with, and the reading of a `[` against the token ahead of
//! it.

use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::source::Source;

/// The characters a bracket closes with, the char-level counterpart to
/// [`is_closer`].
pub(crate) const CLOSERS: [char; 3] = [')', ']', '}'];

/// The characters a bracket opens with, the char-level counterpart to
/// [`is_opener`].
pub(crate) const OPENERS: [char; 3] = ['(', '[', '{'];

/// Returns `true` when `kind` is a closing bracket `)` `]` `}`.
pub(crate) fn is_closer(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace)
}

/// Returns `true` when the gap between a token of `kind` and one of
/// `next` is padding sitting directly inside a bracket delimiter, the
/// run `strip-stranded-padding` deletes. A trivia neighbor on either side
/// leaves the gap alone.
pub(crate) fn is_delimiter_padding(kind: TokenKind, next: TokenKind) -> bool {
    (is_opener(kind) && !next.is_trivia()) || (is_closer(next) && !kind.is_trivia())
}

/// Returns `true` when `kind` opens an f-string or a t-string, the
/// counterpart to `TokenKind::is_interpolated_string_end`.
pub(crate) fn is_interpolated_string_start(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::FStringStart | TokenKind::TStringStart)
}

/// Returns `true` when `kind` is an opening bracket `(` `[` `{`.
pub(crate) fn is_opener(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace)
}

/// The start of every bracket `tokens` leave open once read in order,
/// innermost last.
pub(crate) fn open_brackets<'t>(tokens: impl IntoIterator<Item = &'t Token>) -> Vec<TextSize> {
    let mut open = Vec::new();
    for token in tokens {
        if is_opener(token.kind()) {
            open.push(token.start());
        } else if is_closer(token.kind()) {
            open.pop();
        }
    }
    open
}

/// The tokens of `source` opening inside `range`, a token straddling
/// its start left out.
pub(crate) fn tokens_within(source: &Source, range: TextRange) -> impl Iterator<Item = &Token> {
    source
        .tokens_overlapping(range)
        .filter(move |token| range.contains(token.start()))
}

/// Returns `true` when the `[` at `offset` subscripts the expression
/// ahead of it rather than opening a list, read off the nearest code
/// token before it: a closer, a name, a literal, or a soft keyword used
/// as a name subscripts, whereas an operator, a keyword, or a line start
/// opens a list.
pub(crate) fn opens_subscript(tokens: &Tokens, offset: TextSize) -> bool {
    tokens
        .before(offset)
        .iter()
        .rfind(|token| !token.kind().is_trivia())
        .is_some_and(|prev| {
            let kind = prev.kind();
            is_closer(kind)
                || !(kind.is_operator()
                    || kind.is_any_newline()
                    || matches!(kind, TokenKind::Indent | TokenKind::Dedent)
                    || (kind.is_non_soft_keyword() && !kind.is_singleton()))
        })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{at, parse};

    #[rstest]
    #[case(TokenKind::Rpar, true)]
    #[case(TokenKind::Rsqb, true)]
    #[case(TokenKind::Rbrace, true)]
    #[case(TokenKind::Lpar, false)]
    #[case(TokenKind::Name, false)]
    fn is_closer_flags_closing_brackets(#[case] kind: TokenKind, #[case] expected: bool) {
        assert_eq!(is_closer(kind), expected);
    }

    #[rstest]
    #[case(TokenKind::FStringStart, true)]
    #[case(TokenKind::TStringStart, true)]
    #[case(TokenKind::FStringEnd, false)]
    #[case(TokenKind::String, false)]
    fn is_interpolated_string_start_flags_the_two_openers(
        #[case] kind: TokenKind,
        #[case] expected: bool,
    ) {
        assert_eq!(is_interpolated_string_start(kind), expected);
    }

    #[rstest]
    #[case(TokenKind::Lpar, true)]
    #[case(TokenKind::Lsqb, true)]
    #[case(TokenKind::Lbrace, true)]
    #[case(TokenKind::Rpar, false)]
    #[case(TokenKind::Name, false)]
    fn is_opener_flags_opening_brackets(#[case] kind: TokenKind, #[case] expected: bool) {
        assert_eq!(is_opener(kind), expected);
    }

    #[rstest]
    #[case::after_a_closer("x = f(a)[0]\n", true)]
    #[case::after_a_name("x = y[0]\n", true)]
    #[case::after_a_string("x = 'ab'[0]\n", true)]
    #[case::after_a_soft_keyword("x = type[int]\n", true)]
    #[case::after_an_assignment("x = [0]\n", false)]
    #[case::after_a_comma("x = f(a, [0])\n", false)]
    #[case::after_a_keyword("x = a in [0]\n", false)]
    #[case::at_a_line_start("[a] = b\n", false)]
    fn opens_subscript_reads_the_token_ahead_of_the_bracket(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(
            opens_subscript(source.tokens(), at(src, "[").start()),
            expected
        );
    }
}

//! Token-kind predicates over the bracket delimiters and the
//! interpolated-string openers.

use ruff_python_ast::token::TokenKind;

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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
}

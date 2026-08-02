//! Bracket-delimiter predicates over the token stream, the pair a rule
//! reads to find a delimiter or to track how deep a token sits.

use ruff_python_ast::token::TokenKind;

/// Returns `true` when `kind` is a closing bracket `)` `]` `}`.
pub(crate) fn is_closer(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace)
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
    #[case(TokenKind::Lpar, true)]
    #[case(TokenKind::Lsqb, true)]
    #[case(TokenKind::Lbrace, true)]
    #[case(TokenKind::Rpar, false)]
    #[case(TokenKind::Name, false)]
    fn is_opener_flags_opening_brackets(#[case] kind: TokenKind, #[case] expected: bool) {
        assert_eq!(is_opener(kind), expected);
    }
}

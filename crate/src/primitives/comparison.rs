//! Comparison-operator primitives shared across the rules that read a
//! `CmpOp` back against the source text.

use ruff_python_ast::{CmpOp, token::TokenKind};

/// Maps every `CmpOp` to the lexer token that opens it. A compound
/// operator (`is not`, `not in`) opens on its first keyword, so an
/// alignment column anchors there and a single-token operator's whole
/// span is the returned token's range.
pub(crate) const fn opening_token_kind(op: CmpOp) -> TokenKind {
    match op {
        CmpOp::Eq => TokenKind::EqEqual,
        CmpOp::Gt => TokenKind::Greater,
        CmpOp::GtE => TokenKind::GreaterEqual,
        CmpOp::In => TokenKind::In,
        CmpOp::Is | CmpOp::IsNot => TokenKind::Is,
        CmpOp::Lt => TokenKind::Less,
        CmpOp::LtE => TokenKind::LessEqual,
        CmpOp::NotEq => TokenKind::NotEqual,
        CmpOp::NotIn => TokenKind::Not,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(CmpOp::Eq, TokenKind::EqEqual)]
    #[case(CmpOp::Gt, TokenKind::Greater)]
    #[case(CmpOp::GtE, TokenKind::GreaterEqual)]
    #[case(CmpOp::In, TokenKind::In)]
    #[case(CmpOp::Is, TokenKind::Is)]
    #[case(CmpOp::IsNot, TokenKind::Is)]
    #[case(CmpOp::Lt, TokenKind::Less)]
    #[case(CmpOp::LtE, TokenKind::LessEqual)]
    #[case(CmpOp::NotEq, TokenKind::NotEqual)]
    #[case(CmpOp::NotIn, TokenKind::Not)]
    fn opening_token_kind_covers_every_variant(#[case] op: CmpOp, #[case] expected: TokenKind) {
        assert_eq!(opening_token_kind(op), expected);
    }
}

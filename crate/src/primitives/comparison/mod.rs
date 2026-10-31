//! Comparison-operator primitives shared across the rules that read a
//! `CmpOp` back against the source text, among them the padding a join
//! onto one row narrows ahead of each operator.

use ruff_diagnostics::Edit;
use ruff_python_ast::{CmpOp, Expr, ExprCompare, token::TokenKind};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        aligner::line_gap_before,
        walk::{Interpolations, filter_map_within_expr},
    },
    source::Source,
};

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

/// The edits a join onto one row makes ahead of `compare`'s operators,
/// each narrowing to one space a run of two or more spaces that opens
/// mid-row and that no edit in `held` overlaps.
pub(crate) fn narrowing_edits(source: &Source, compare: &ExprCompare, held: &[Edit]) -> Vec<Edit> {
    compare
        .iter()
        .filter_map(|(left, &op, right)| {
            let at = source.first_token_offset_in_range(
                TextRange::new(left.end(), right.start()),
                |token| token.kind() == opening_token_kind(op),
            )?;
            let gap = line_gap_before(source, at);
            let mid_row = source.text().line_start(at) < gap.start();
            let covered = held
                .iter()
                .any(|edit| edit.range().intersect(gap).is_some_and(|r| !r.is_empty()));
            (mid_row && gap.len() > TextSize::of(' ') && !covered)
                .then(|| Edit::range_replacement(" ".to_owned(), gap))
        })
        .collect()
}

/// The [`narrowing_edits`] of every comparison inside `expr`, ascending
/// by start, leaving a replacement field's interior unread, and none
/// where `expr` sits on one row.
pub(crate) fn narrowing_edits_within(source: &Source, expr: &Expr, held: &[Edit]) -> Vec<Edit> {
    if !source.contains_line_break(expr.range()) {
        return Vec::new();
    }
    let mut edits = filter_map_within_expr(expr, Interpolations::Skip, |expr| {
        expr.as_compare_expr()
            .map(|compare| narrowing_edits(source, compare, held))
    })
    .concat();
    edits.sort_by_key(Ranged::start);
    edits
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

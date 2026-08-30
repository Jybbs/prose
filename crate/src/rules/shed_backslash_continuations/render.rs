//! Turns a shed decision into the edits joining or wrapping its run.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, find_node::covering_node, token::TokenKind};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use super::*;
use crate::primitives::inline::display_width;

/// Replaces each gap in `run` with its join text, folding the physical
/// lines the run continues onto one.
pub(super) fn join_edits(source: &Source, run: &[Gap]) -> Vec<Edit> {
    run.iter()
        .filter_map(|gap| narrowed_replacement(source, gap.range, gap.join.to_owned()))
        .collect()
}

/// The text a join leaves between `token` and `next`, empty where the
/// pair abuts with no space and one space otherwise.
pub(super) fn join_text(token: TokenKind, next: TokenKind) -> &'static str {
    let abuts = token == TokenKind::Dot
        || next.is_any_newline()
        || matches!(
            next,
            TokenKind::Colon | TokenKind::Comma | TokenKind::Dot | TokenKind::Semi
        )
        || (matches!(next, TokenKind::Lpar | TokenKind::Lsqb) && ends_atom(token));
    if abuts { "" } else { " " }
}

/// The display width of the physical line `edits` fold `span` onto,
/// measured from the opening line's first column through the closing
/// line's last.
pub(super) fn joined_width(source: &Source, span: TextRange, edits: &[Edit]) -> usize {
    display_width(&apply_inline_edits(
        source,
        source.text().lines_range(span),
        edits,
    ))
}

/// Drops every backslash in `gap` along with the whitespace ahead of
/// it, keeping the line breaks the gap carries. Returns `None` where
/// the gap already reads that way.
pub(super) fn stripped_edit(source: &Source, gap: TextRange) -> Option<Edit> {
    narrowed_replacement(source, gap, stripped_gap(source, gap))
}

/// The outermost expression spanning `run` and the edits dropping the
/// run's backslashes inside it, keeping every break, the pair around
/// that expression left to the caller so runs sharing it take one.
/// Returns `None` where no expression spans the run or the wrapped form
/// reparses to a different tree.
pub(super) fn wrap_edits(
    source: &Source,
    span: TextRange,
    run: &[Gap],
) -> Option<(TextRange, Vec<Edit>)> {
    let root = AnyNodeRef::from(source.ast());
    let wrapped = covering_node(root, span)
        .find_last(AnyNodeRef::is_expression)
        .ok()?
        .node()
        .range();
    let edits: Vec<Edit> = run
        .iter()
        .filter_map(|gap| stripped_edit(source, gap.range))
        .collect();
    let candidate = format!("({})", apply_inline_edits(source, wrapped, &edits));
    splice_preserves_tree(source, wrapped, &candidate).then_some((wrapped, edits))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case(TokenKind::Plus, TokenKind::Int, " ")]
    #[case(TokenKind::Equal, TokenKind::Lsqb, " ")]
    #[case(TokenKind::Name, TokenKind::Dot, "")]
    #[case(TokenKind::Dot, TokenKind::Name, "")]
    #[case(TokenKind::Name, TokenKind::Lsqb, "")]
    #[case(TokenKind::Name, TokenKind::Lpar, "")]
    #[case(TokenKind::Name, TokenKind::Comma, "")]
    #[case(TokenKind::Int, TokenKind::Newline, "")]
    fn join_text_spaces_only_where_the_pair_takes_one(
        #[case] token: TokenKind,
        #[case] next: TokenKind,
        #[case] expected: &str,
    ) {
        assert_eq!(join_text(token, next), expected);
    }

    #[rstest]
    #[case("x = 1 + \\\n    2\n", "x = 1 + 2")]
    #[case("x = 1 + \\\n    2  # note\n", "x = 1 + 2  # note")]
    fn joined_width_measures_the_line_the_run_produces(#[case] src: &str, #[case] expected: &str) {
        let source = parse(src);
        let gaps = continuation_gaps(&source);
        let edits = join_edits(&source, &gaps);
        assert_eq!(
            joined_width(&source, blocks_span(&gaps), &edits),
            expected.len(),
        );
    }

    #[test]
    fn wrap_edits_declines_a_break_no_expression_spans() {
        let source = parse("import alpha, \\\n    beta\n");
        let gaps = continuation_gaps(&source);
        assert!(wrap_edits(&source, blocks_span(&gaps), &gaps).is_none());
    }
}

//! Source ranges for expression nodes and comma-separated lists.

use ruff_python_ast::{
    AnyNodeRef, Expr, ExprRef, StmtFunctionDef,
    token::{Tokens, parenthesized_range},
};
use ruff_text_size::{Ranged, TextRange};

/// The span deleting `target` leaves in a comma-separated list whose
/// members `ordered` yields in source order, where `target` covers one
/// member or a contiguous run of them. Reaches forward to the following
/// member's start where one exists and back to the preceding member's
/// end otherwise.
pub(crate) fn member_deletion_span<M: Ranged>(
    ordered: impl IntoIterator<Item = M>,
    target: TextRange,
) -> TextRange {
    let mut members = ordered.into_iter().map(|member| member.range());
    let preceding = members
        .by_ref()
        .take_while(|member| member.start() < target.start())
        .last();
    match (
        preceding,
        members.find(|member| member.start() >= target.end()),
    ) {
        (_, Some(following)) => TextRange::new(target.start(), following.start()),
        (Some(preceding), None) => TextRange::new(preceding.end(), target.end()),
        (None, None) => unreachable!("invariant: a deleted member keeps at least one sibling"),
    }
}

/// Returns `expr`'s range widened to the explicit parentheses recovered
/// against `parent`, falling back to the bare expression range when none
/// enclose it.
pub(crate) fn paren_aware_range(expr: ExprRef, parent: AnyNodeRef, tokens: &Tokens) -> TextRange {
    parenthesized_range(expr, parent, tokens).unwrap_or_else(|| expr.range())
}

/// Returns the paren-aware range of `function`'s return annotation,
/// recovered against the function def.
pub(crate) fn return_annotation_range(
    annotation: &Expr,
    function: &StmtFunctionDef,
    tokens: &Tokens,
) -> TextRange {
    paren_aware_range(annotation.into(), function.into(), tokens)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::range;

    #[rstest]
    #[case(0, 10, 0, 12)]
    #[case(6, 16, 4, 16)]
    fn member_deletion_span_steps_over_a_run_target(
        #[case] target_start: u32,
        #[case] target_end: u32,
        #[case] start: u32,
        #[case] end: u32,
    ) {
        let members = [range(0, 4), range(6, 10), range(12, 16)];
        assert_eq!(
            member_deletion_span(members, range(target_start, target_end)),
            range(start, end),
        );
    }

    #[rstest]
    #[case(0, 0, 6)]
    #[case(1, 6, 12)]
    #[case(2, 10, 16)]
    fn member_deletion_span_takes_the_separator_on_the_surviving_side(
        #[case] index: usize,
        #[case] start: u32,
        #[case] end: u32,
    ) {
        let members = [range(0, 4), range(6, 10), range(12, 16)];
        assert_eq!(
            member_deletion_span(members, members[index]),
            range(start, end),
        );
    }
}

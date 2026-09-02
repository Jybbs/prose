//! Member constructors for the `:` alignment contexts: dict items,
//! annotated assignments in any scope, annotated function parameters,
//! and `match` arm cases. Each member carries the post-colon
//! `value_gap` an aligned or stripped row rewrites to one space, left
//! `None` where match arms defer to `align_match_case`.

use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, DictItem, ExprDict, ExprRef, MatchCase, Parameters, Stmt,
    token::TokenKind,
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{primitives::aligner, rules::RuleId, source::Source};

/// Walks `body`, qualifying each statement through `annotated_assignment`,
/// and returns one group per run of contiguous line-adjacent
/// annotated-assignment statements. A single-line row held for `rule`
/// drops out as a transparent hole per [`aligner::line_adjacent_groups`].
pub(super) fn annotated_assignment_groups(
    source: &Source,
    rule: RuleId,
    body: &[Stmt],
) -> Vec<Vec<aligner::Member>> {
    aligner::line_adjacent_groups(source, body, rule, |s| annotated_assignment(source, s))
}

/// Returns one group per run of consecutive-line `key: value` entries
/// in `d`. A trailing comment on an entry stays with it and keeps the
/// run going, whereas a standalone comment line or a blank line between
/// two entries closes the active run and starts a fresh one, so each
/// run aligns independently. `**spread` entries skip the colon scan but
/// do not break the run, matching the long-standing rule that an
/// unpacking passes alignment through. A keyed entry whose `:` crosses a
/// line break instead closes the run, so its neighbors do not align a
/// column across the stranded colon.
pub(super) fn dict_member_groups(
    source: &Source,
    rule: RuleId,
    dict: &ExprDict,
) -> Vec<Vec<aligner::Member>> {
    aligner::adjacent_member_groups(source, &dict.items, false, |item| {
        // A `**spread` (no key) or a skip-held entry joins no group yet
        // bridges the run, so the entries on either side align as one block.
        if item.key.is_none() || aligner::is_held(source, rule, item.start()) {
            return aligner::Slot::Bridge;
        }
        // A keyed entry whose colon sits on a later line carries no
        // single-line anchor, so it breaks the run rather than stranding
        // its colon inside a column its neighbors share.
        dict_item(source, dict, item).into()
    })
}

/// Builds an alignment member for a `match` arm, anchored on the
/// `:` between the pattern (or its `if` guard) and the arm body's
/// first statement.
pub(crate) fn match_case(source: &Source, case: &MatchCase) -> Option<aligner::Member> {
    let pre_colon_end = match_case_pre_colon_end(case);
    let body_start = case.body.first()?.start();
    aligner::line_anchored_member_between(
        source,
        TextRange::new(case.pattern.start(), pre_colon_end),
        body_start,
        TokenKind::Colon,
    )
}

/// Returns one alignment member per `case` arm in `cases`.
pub(super) fn match_case_members(source: &Source, cases: &[MatchCase]) -> Vec<aligner::Member> {
    cases.iter().filter_map(|c| match_case(source, c)).collect()
}

/// The offset where a `match` arm's pre-colon left-hand side ends, the
/// guard's end when the arm is guarded and the pattern's end otherwise.
pub(crate) fn match_case_pre_colon_end(case: &MatchCase) -> TextSize {
    case.guard
        .as_deref()
        .map_or(case.pattern.end(), Ranged::end)
}

/// Walks `params` in source order and returns one group per run of
/// contiguous annotated parameters, splitting at every unannotated
/// parameter. A parameter skip-suppressed for `rule` drops out of its
/// group as a transparent hole, leaving its neighbors to align.
pub(super) fn parameter_groups(
    source: &Source,
    rule: RuleId,
    params: &Parameters,
) -> Vec<Vec<aligner::Member>> {
    aligner::parameter_split_groups(params, |p| parameter(source, p))
        .into_iter()
        .map(|group| aligner::retain_unheld(source, rule, group))
        .collect()
}

/// Builds an alignment member for an annotated assignment, anchored on
/// the `:` between target and annotation. Returns `None` for any other
/// statement shape.
fn annotated_assignment(source: &Source, stmt: &Stmt) -> Option<aligner::Member> {
    let ann = stmt.as_ann_assign_stmt()?;
    colon_member(
        source,
        ann.target.range(),
        ann.annotation.as_ref().into(),
        ann.into(),
    )
}

/// Builds a `:`-anchored alignment member whose left-hand side is `lhs`,
/// scanning between `lhs.end()` and `value`'s parenthesis-aware start
/// recovered against `parent`, leaving a colon inside the left-hand side
/// unanchored. Rejects a colon that does not share `lhs`'s opening line.
/// `value_gap` runs from just past the colon to that start.
fn colon_member(
    source: &Source,
    lhs: TextRange,
    value: ExprRef,
    parent: AnyNodeRef,
) -> Option<aligner::Member> {
    let value_start = source.paren_aware_range(value, parent).start();
    let member = aligner::line_anchored_member_between(source, lhs, value_start, TokenKind::Colon)?;
    Some(member.with_value_gap(TextSize::of(':'), value_start))
}

/// Builds an alignment member for a `key: value` dict entry, anchored
/// on the `:` between key and value. Returns `None` for `**spread`
/// entries that have no key.
fn dict_item(source: &Source, dict: &ExprDict, item: &DictItem) -> Option<aligner::Member> {
    let key = item.key.as_ref()?;
    colon_member(source, key.range(), (&item.value).into(), dict.into())
}

/// Builds an alignment member for an annotated function parameter,
/// anchored on the `:` between name and annotation. Returns `None` for
/// unannotated parameters, signaling a group break to callers.
fn parameter(source: &Source, param: AnyParameterRef<'_>) -> Option<aligner::Member> {
    let annotation = param.annotation()?;
    colon_member(
        source,
        param.name().range(),
        annotation.into(),
        param.as_parameter().into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{first_value, parse};

    #[test]
    fn annotated_assignment_rejects_cross_line_colon() {
        // The target ends on its own line and the annotation `:` opens
        // the next.
        let source = parse("class C:\n    x \\\n        : int\n");
        let class = source.ast().body[0].as_class_def_stmt().expect("class");
        assert!(annotated_assignment(&source, &class.body[0]).is_none());
    }

    #[test]
    fn dict_item_rejects_cross_line_key() {
        // The key ends on its own line and the `:` opens the next, so the
        // entry yields no alignable member.
        let source = parse("d = {\n    k\n    : v,\n}\n");
        let dict = first_value(&source).as_dict_expr().expect("dict");
        assert!(dict_item(&source, dict, &dict.items[0]).is_none());
    }

    #[test]
    fn match_case_rejects_multiline_pattern() {
        // The pattern spans several lines, placing the `:` off the line
        // where the pattern opens.
        let source = parse("match x:\n    case (\n        1,\n        2,\n    ):\n        y\n");
        let m = source.ast().body[0].as_match_stmt().expect("match");
        assert!(match_case(&source, &m.cases[0]).is_none());
    }

    #[test]
    fn parameter_rejects_cross_line_colon() {
        // The parameter name ends on its own line and the annotation `:`
        // opens the next.
        let source = parse("def f(\n    a\n    : int,\n):\n    pass\n");
        let func = source.ast().body[0].as_function_def_stmt().expect("def");
        let param = func.parameters.iter_source_order().next().expect("param");
        assert!(parameter(&source, param).is_none());
    }
}

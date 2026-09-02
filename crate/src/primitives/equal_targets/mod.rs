//! Member constructors for the `=`-anchored alignment contexts:
//! single-target assignments, augmented assignments, initialized
//! annotated assignments, exploded-call keyword arguments, and
//! annotated parameter defaults. `align_equals` consumes them to emit
//! alignment edits, and the `reserve` primitive consumes them to predict
//! the column `align_equals` shifts a value to for both layout rules.

use itertools::Itertools;
use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, ArgOrKeyword, ExprCall, ExprRef, Stmt, token::TokenKind,
};
use ruff_source_file::OneIndexed;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::{primitives::aligner, rule::RuleId, source::Source};

/// Returns the alignment member for an annotated `x: int = 1`, plain
/// `x = 1`, or augmented `x += 1` statement, measuring the left-hand
/// side paren-aware. `None` for any other shape or when the span up to
/// the operator breaks across lines.
pub(crate) fn assignment(source: &Source, stmt: &Stmt) -> Option<aligner::Member> {
    match stmt {
        Stmt::AnnAssign(a) => {
            let value = a.value.as_deref()?;
            let annotation = source.paren_aware_range(a.annotation.as_ref().into(), a.into());
            equal_member(
                source,
                a.target.range().cover(annotation),
                value.into(),
                a.into(),
            )
        }
        Stmt::Assign(a) => {
            let [target] = a.targets.as_slice() else {
                return None;
            };
            equal_member(
                source,
                source.paren_aware_range(target.into(), a.into()),
                a.value.as_ref().into(),
                a.into(),
            )
        }
        Stmt::AugAssign(a) => {
            let op = a.op.as_str();
            let target_range = source.paren_aware_range(a.target.as_ref().into(), a.into());
            let value_start = source
                .paren_aware_range(a.value.as_ref().into(), a.into())
                .start();
            let member = aligner::range_anchored_member_single_line(
                source,
                target_range,
                TextRange::new(target_range.end(), value_start),
                |t| t.kind().as_augmented_assign_operator().is_some(),
                op.len(),
            )?;
            // `op` is the binary form (`+`), so the augmented operator
            // runs one column longer for its trailing `=`.
            let op_len = TextSize::of(op) + TextSize::of('=');
            Some(member.with_value_gap(op_len, value_start))
        }
        _ => None,
    }
}

/// Groups `call`'s keyword arguments into the runs `align_equals`
/// aligns, empty for a single-line call whose keywords stay condensed. A
/// positional argument, a `**` unpacking, or a keyword sharing its
/// physical line with another argument ends the active run, so a
/// condensed keyword keeps its tight `name=value`. A skip-held keyword
/// bridges the run without joining. `break_after_multiline` closes a run
/// after a keyword whose own value spans lines, so a wrapped value does
/// not drag later keywords into its group.
pub(crate) fn keyword_groups(
    source: &Source,
    rule: RuleId,
    call: &ExprCall,
    break_after_multiline: bool,
) -> Vec<Vec<aligner::Member>> {
    if !source.contains_line_break(call.arguments.range()) {
        return Vec::new();
    }
    let shared_lines: FxHashSet<OneIndexed> = call
        .arguments
        .iter_source_order()
        .map(|a| source.line_index(a.start()))
        .duplicates()
        .collect();
    aligner::adjacent_member_groups(
        source,
        call.arguments.iter_source_order(),
        break_after_multiline,
        |arg| {
            let Some(member) = keyword(source, arg) else {
                return aligner::Slot::Break;
            };
            if shared_lines.contains(&source.line_index(member.line_start)) {
                return aligner::Slot::Break;
            }
            member.slot(source, rule)
        },
    )
}

/// Returns the alignment member for an annotated function parameter
/// carrying a default value, or `None` for any other shape. Width spans
/// the parameter name through the annotation's paren-aware end, and the
/// value-side gap is recovered against the parameter-with-default node so
/// a parenthesized default keeps its `(`.
pub(crate) fn parameter(source: &Source, param: AnyParameterRef<'_>) -> Option<aligner::Member> {
    let AnyParameterRef::NonVariadic(with_default) = param else {
        return None;
    };
    let annotation = param.annotation()?;
    let default = param.default()?;
    let annotation_end = source
        .paren_aware_range(annotation.into(), param.as_parameter().into())
        .end();
    equal_member(
        source,
        TextRange::new(param.name().start(), annotation_end),
        default.into(),
        with_default.into(),
    )
}

/// Builds an `=`-anchored member with `target` as the LHS span,
/// anchoring the `=` between `target.end()` and `value`'s
/// parenthesis-aware start so the value-side gap stops before any
/// wrapping `(`.
fn equal_member(
    source: &Source,
    target: TextRange,
    value: ExprRef,
    parent: AnyNodeRef,
) -> Option<aligner::Member> {
    let value_start = source.paren_aware_range(value, parent).start();
    let member = aligner::range_anchored_member_single_line(
        source,
        target,
        TextRange::new(target.end(), value_start),
        |t| t.kind() == TokenKind::Equal,
        0,
    )?;
    Some(member.with_value_gap(TextSize::of('='), value_start))
}

/// Returns the alignment member for a `name=value` keyword argument, or
/// `None` for a positional argument or a `**` unpacking. A keyword whose
/// value spans lines still qualifies, since its `=` sits on the keyword's
/// first line where [`equal_member`] anchors it.
fn keyword(source: &Source, arg: ArgOrKeyword<'_>) -> Option<aligner::Member> {
    let ArgOrKeyword::Keyword(keyword) = arg else {
        return None;
    };
    let name = keyword.arg.as_ref()?;
    equal_member(
        source,
        name.range(),
        (&keyword.value).into(),
        keyword.into(),
    )
}

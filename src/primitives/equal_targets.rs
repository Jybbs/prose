//! Member constructors for the `=`-anchored alignment contexts:
//! single-target assignments, augmented assignments, initialized
//! annotated assignments, exploded-call keyword arguments, and
//! annotated parameter defaults. `align_equals` consumes them to emit
//! alignment edits, and `collection_layout` consumes them to reserve
//! the column `align_equals` shifts a value to.

use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, ArgOrKeyword, ExprCall, ExprRef, Stmt, token::TokenKind,
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{primitives::aligner, rule::RuleId, source::Source};

/// An `=`-anchored alignment member paired with `value_gap`, the span
/// between the operator and the value that an aligned run rewrites to
/// one space.
#[derive(Clone, Copy)]
pub(crate) struct EqualMember {
    pub(crate) member: aligner::Member,
    pub(crate) value_gap: TextRange,
}

impl EqualMember {
    /// Pairs `member` with the value-side gap running from just past its
    /// `op_len`-wide operator to the parenthesis-aware `value_start`.
    fn new(member: aligner::Member, op_len: TextSize, value_start: TextSize) -> Self {
        Self {
            member,
            value_gap: TextRange::new(member.gap.end() + op_len, value_start),
        }
    }

    /// This member's alignment slot, bridging the run when its row is
    /// skip-held for `rule` so neighbors align around it.
    pub(crate) fn slot(self, source: &Source, rule: RuleId) -> aligner::Slot<Self> {
        if aligner::is_held(source, rule, self.member.line_start) {
            aligner::Slot::Bridge
        } else {
            aligner::Slot::Member(self)
        }
    }

    /// The value's parenthesis-aware start, where the `=`-side gap closes.
    pub(crate) fn value_start(self) -> TextSize {
        self.value_gap.end()
    }
}

/// Returns the alignment member for an annotated `x: int = 1`, plain
/// `x = 1`, or augmented `x += 1` statement, measuring the left-hand
/// side paren-aware. `None` for any other shape or when the span up to
/// the operator breaks across lines.
pub(crate) fn assignment(source: &Source, stmt: &Stmt) -> Option<EqualMember> {
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
            Some(EqualMember::new(member, op_len, value_start))
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
) -> Vec<Vec<EqualMember>> {
    if !source.contains_line_break(call.arguments.range()) {
        return Vec::new();
    }
    let arg_lines: Vec<_> = call
        .arguments
        .arguments_source_order()
        .map(|a| source.line_index(a.start()))
        .collect();
    aligner::adjacent_member_groups(
        source,
        call.arguments.arguments_source_order(),
        break_after_multiline,
        |arg| {
            let Some(member) = keyword(source, arg) else {
                return aligner::Slot::Break;
            };
            let line = source.line_index(member.member.line_start);
            if arg_lines.iter().filter(|&&l| l == line).count() > 1 {
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
pub(crate) fn parameter(source: &Source, param: AnyParameterRef<'_>) -> Option<EqualMember> {
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
) -> Option<EqualMember> {
    let value_start = source.paren_aware_range(value, parent).start();
    let member = aligner::range_anchored_member_single_line(
        source,
        target,
        TextRange::new(target.end(), value_start),
        |t| t.kind() == TokenKind::Equal,
        0,
    )?;
    Some(EqualMember::new(member, TextSize::of('='), value_start))
}

/// Returns the alignment member for a `name=value` keyword argument, or
/// `None` for a positional argument or a `**` unpacking. A keyword whose
/// value spans lines still qualifies, since its `=` sits on the keyword's
/// first line where [`equal_member`] anchors it.
fn keyword(source: &Source, arg: ArgOrKeyword<'_>) -> Option<EqualMember> {
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

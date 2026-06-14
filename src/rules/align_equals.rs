//! Aligns the `=` character vertically across runs of same-indent,
//! line-adjacent `Stmt::Assign` (single-target), `Stmt::AugAssign`, and
//! `Stmt::AnnAssign` (with initializer) statements, across annotated
//! function-parameter defaults, and across an exploded call's keyword
//! arguments. Chained assignments, initializer-less annotations,
//! unannotated parameter defaults, and single-line signatures or calls
//! are skipped. Each aligned row reads as `name = value`: the name side
//! pads to the shared column, collapsing to one space for a lone row or
//! singleton sub-group, and the value side collapses to one space after
//! the operator. The value-side rewrite stops at the value's
//! parenthesis-inclusive start and sits out when the value falls on a
//! later line, so a wrapped or continued value keeps its placement.
//! `+=` rows place `+` one column before the shared `=` column rather
//! than pushing the `=` right. Parameter widths reflect the
//! post-`align_colons` source.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, ArgOrKeyword, Expr, ExprCall, ExprRef, Parameters, Stmt,
    token::TokenKind,
    visitor::{Visitor as AstVisitor, walk_body, walk_expr, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{aligner, range::paren_aware_range},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignEquals {
    settings: aligner::Settings,
}

impl AlignEquals {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_equals),
        }
    }
}

impl Rule for AlignEquals {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// An `=`-anchored alignment member paired with `value_gap`, the span
/// between the operator and the value that an aligned run rewrites to
/// one space.
#[derive(Clone, Copy)]
struct EqualMember {
    member: aligner::Member,
    value_gap: TextRange,
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
}

struct Visitor<'a> {
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Emits the unheld members of `group` as one fix when at least two
    /// survive to align, padding each name side to the shared column and
    /// folding in the [value-side gaps](Self::value_gaps). A lone
    /// surviving member emits nothing, leaving a single defaulted
    /// parameter untouched.
    fn emit_aligned(&mut self, group: &[EqualMember]) {
        let kept = self
            .walker
            .retain_unheld(group.iter().copied(), |m| m.member.line_start);
        let members: Vec<aligner::Member> = kept.iter().map(|m| m.member).collect();
        if aligner::is_alignment_candidate(&members) {
            let gaps = self.value_gaps(&kept);
            self.walker.emit_group_with_gaps(&members, gaps);
        }
    }

    /// Emits `group` as one fix: the aligner's column when the members
    /// align on distinct lines, a one-space name-side buffer otherwise,
    /// plus the [value-side gaps](Self::value_gaps) so every row reads as
    /// `name = value`.
    fn emit_equal_group(&mut self, group: &[EqualMember]) {
        let source = self.walker.source;
        let members: Vec<aligner::Member> = group.iter().map(|m| m.member).collect();
        let name_edits = if aligner::is_alignment_candidate(&members) {
            self.walker.group_edits(&members)
        } else {
            group
                .iter()
                .filter_map(|m| aligner::space_padding_edit(source, m.member.gap, 1))
                .collect()
        };
        let gaps = self.value_gaps(group);
        self.walker.push_with_gaps(name_edits, gaps);
    }

    /// Builds an `=`-anchored member with `target` as the LHS span,
    /// anchoring the `=` between `target.end()` and `value`'s
    /// parenthesis-aware start so the value-side gap stops before any
    /// wrapping `(`.
    fn equal_member(
        &self,
        target: TextRange,
        value: ExprRef,
        parent: AnyNodeRef,
    ) -> Option<EqualMember> {
        let value_start = self.paren_aware(value, parent).start();
        let member = aligner::range_anchored_member_single_line(
            self.walker.source,
            target,
            TextRange::new(target.end(), value_start),
            |t| t.kind() == TokenKind::Equal,
            0,
        )?;
        Some(EqualMember::new(member, TextSize::of('='), value_start))
    }

    /// Splits `call`'s arguments into line-adjacent runs of keyword
    /// members. A positional argument, a `**` unpacking, or an interior
    /// comment ends the active run. A keyword whose value spans lines
    /// joins the run, then closes it after itself, so the keywords past
    /// it align as a separate group.
    fn keyword_groups(&self, call: &ExprCall) -> Vec<Vec<EqualMember>> {
        aligner::adjacent_member_groups(
            self.walker.source,
            call.arguments.arguments_source_order(),
            true,
            |arg| match self.qualify_keyword(arg) {
                Some(keyword) if self.walker.is_held(keyword.member.line_start) => {
                    aligner::Slot::Bridge
                }
                Some(keyword) => aligner::Slot::Member(keyword),
                None => aligner::Slot::Break,
            },
        )
    }

    /// `expr`'s range widened to its explicit parentheses, recovered
    /// against `parent`, so a parenthesized left-hand side ends past
    /// its closing `)` instead of leaving the paren inside the gap.
    fn paren_aware(&self, expr: ExprRef, parent: AnyNodeRef) -> TextRange {
        paren_aware_range(expr, parent, self.walker.source.tokens())
    }

    fn process_body(&mut self, body: &[Stmt]) {
        let rule = self.walker.rule;
        for group in
            aligner::line_adjacent_groups(self.walker.source, body, rule, |s| self.qualify(s))
        {
            self.emit_equal_group(&group);
        }
    }

    /// Aligns each line-adjacent run of `call`'s keyword arguments,
    /// padding before each `=` and rewriting the gap after it to one
    /// space. A keyword that stands alone in its group instead takes a
    /// one-space buffer on each side of its `=`, so every exploded
    /// keyword reads as `name = value`. A single-line call is left
    /// untouched.
    fn process_call(&mut self, call: &ExprCall) {
        let source = self.walker.source;
        if !source.contains_line_break(call.arguments.range()) {
            return;
        }
        for group in self.keyword_groups(call) {
            self.emit_equal_group(&group);
        }
    }

    /// Walks `params` through [`aligner::adjacent_member_groups`] with
    /// [`Self::qualify_parameter`], emitting an alignment pass for each
    /// run of defaulted parameters. A multi-line default closes the run
    /// after it, so the parameters past it align as a separate group,
    /// mirroring an exploded call's keyword runs.
    fn process_parameters(&mut self, params: &Parameters) {
        let groups = aligner::adjacent_member_groups(
            self.walker.source,
            params.iter_source_order(),
            true,
            |param| self.qualify_parameter(param).into(),
        );
        for group in groups {
            self.emit_aligned(&group);
        }
    }

    /// Returns the alignment member for an annotated `x: int = 1`, plain
    /// `x = 1`, or augmented `x += 1` statement, measuring the left-hand
    /// side [paren-aware](Self::paren_aware). `None` for any other shape
    /// or when the span up to the operator breaks across lines.
    fn qualify(&self, stmt: &Stmt) -> Option<EqualMember> {
        match stmt {
            Stmt::AnnAssign(a) => {
                let value = a.value.as_deref()?;
                let annotation = self.paren_aware(a.annotation.as_ref().into(), a.into());
                self.equal_member(a.target.range().cover(annotation), value.into(), a.into())
            }
            Stmt::Assign(a) => {
                let [target] = a.targets.as_slice() else {
                    return None;
                };
                self.equal_member(
                    self.paren_aware(target.into(), a.into()),
                    a.value.as_ref().into(),
                    a.into(),
                )
            }
            Stmt::AugAssign(a) => {
                let op = a.op.as_str();
                let target_range = self.paren_aware(a.target.as_ref().into(), a.into());
                let value_start = self.paren_aware(a.value.as_ref().into(), a.into()).start();
                let member = aligner::range_anchored_member_single_line(
                    self.walker.source,
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

    /// Returns the alignment member for a `name=value` keyword argument,
    /// or `None` for a positional argument or a `**` unpacking. A keyword
    /// whose value spans lines still qualifies, since its `=` sits on the
    /// keyword's first line where [`Self::equal_member`] anchors it.
    fn qualify_keyword(&self, arg: ArgOrKeyword<'_>) -> Option<EqualMember> {
        let ArgOrKeyword::Keyword(keyword) = arg else {
            return None;
        };
        let name = keyword.arg.as_ref()?;
        self.equal_member(name.range(), (&keyword.value).into(), keyword.into())
    }

    /// Returns the alignment member for an annotated function parameter
    /// carrying a default value, or `None` for any other shape. Width
    /// spans the parameter name through the annotation's
    /// [paren-aware](Self::paren_aware) end, and the value-side gap is
    /// recovered against the parameter-with-default node so a
    /// parenthesized default keeps its `(`.
    fn qualify_parameter(&self, param: AnyParameterRef<'_>) -> Option<EqualMember> {
        let AnyParameterRef::NonVariadic(with_default) = param else {
            return None;
        };
        let annotation = param.annotation()?;
        let default = param.default()?;
        let annotation_end = self
            .paren_aware(annotation.into(), param.as_parameter().into())
            .end();
        self.equal_member(
            TextRange::new(param.name().start(), annotation_end),
            default.into(),
            with_default.into(),
        )
    }

    /// The value-side gaps for every member in `group` whose value
    /// shares the operator's line. A gap spanning a line break is
    /// dropped, leaving a continued value where the source placed it.
    fn value_gaps(&self, group: &[EqualMember]) -> Vec<TextRange> {
        let source = self.walker.source;
        group
            .iter()
            .filter(|m| !source.contains_line_break(m.value_gap))
            .map(|m| m.value_gap)
            .collect()
    }
}

impl<'a> AstVisitor<'a> for Visitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.process_body(body);
        walk_body(self, body);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.process_call(call);
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_parameters(&fd.parameters);
        }
        walk_stmt(self, stmt);
    }
}

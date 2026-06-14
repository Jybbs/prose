//! Aligns the `=` character vertically across runs of same-indent,
//! line-adjacent `Stmt::Assign` (single-target), `Stmt::AugAssign`, and
//! `Stmt::AnnAssign` (with initializer) statements, across annotated
//! function-parameter defaults, and across an exploded call's keyword
//! arguments. Chained assignments, initializer-less annotations,
//! unannotated parameter defaults, and single-line signatures or calls
//! are skipped. Lone assignment rows and singleton sub-groups still
//! collapse their pre-`=` whitespace to one space, whereas a lone
//! exploded keyword takes a one-space buffer on each side so it reads
//! as `name = value`. `+=` rows place `+` one column
//! before the shared `=` column rather than pushing the `=` right.
//! Parameter widths reflect the post-`align_colons` source.

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

/// A keyword argument's `=`-anchored alignment member paired with
/// `value_gap`, the span between the `=` and the value that an aligned
/// run rewrites to one space.
#[derive(Clone, Copy)]
struct KeywordMember {
    member: aligner::Member,
    value_gap: TextRange,
}

struct Visitor<'a> {
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Builds an `=`-anchored member with `target` as the LHS span and
    /// the search range running from `target.end()` to `value_start`.
    fn equal_member(&self, target: TextRange, value_start: TextSize) -> Option<aligner::Member> {
        aligner::range_anchored_member_single_line(
            self.walker.source,
            target,
            TextRange::new(target.end(), value_start),
            |t| t.kind() == TokenKind::Equal,
            0,
        )
    }

    /// Splits `call`'s arguments into line-adjacent runs of keyword
    /// members. A positional argument, a `**` unpacking, or an interior
    /// comment ends the active run. A keyword whose value spans lines
    /// joins the run, then closes it after itself, so the keywords past
    /// it align as a separate group.
    fn keyword_groups(&self, call: &ExprCall) -> Vec<Vec<KeywordMember>> {
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
        for members in
            aligner::line_adjacent_groups(self.walker.source, body, rule, |s| self.qualify(s))
        {
            self.walker.emit_group(&members);
        }
    }

    /// Aligns each line-adjacent run of `call`'s keyword arguments,
    /// padding before each `=` and rewriting the gap after it to one
    /// space. A run pads its `=` only when its keywords each sit on
    /// their own line at a shared column baseline. A lone keyword, or a
    /// run whose rows open at differing columns, instead takes a
    /// one-space buffer on each side of its `=`, so every exploded
    /// keyword reads as `name = value`. A single-line call and a held
    /// row are left untouched.
    fn process_call(&mut self, call: &ExprCall) {
        let source = self.walker.source;
        if !source.contains_line_break(call.arguments.range()) {
            return;
        }
        for group in self.keyword_groups(call) {
            let members: Vec<aligner::Member> = group.iter().map(|k| k.member).collect();
            let mut edits = if aligner::is_alignment_candidate(source, &members) {
                self.walker.group_edits(&members)
            } else {
                group
                    .iter()
                    .filter_map(|k| aligner::space_padding_edit(source, k.member.gap, 1))
                    .collect()
            };
            edits.extend(
                group
                    .iter()
                    .filter_map(|k| aligner::space_padding_edit(source, k.value_gap, 1)),
            );
            self.walker.push_group(edits);
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
        for members in groups {
            self.walker.emit_unheld(members);
        }
    }

    /// Returns the alignment member for an annotated `x: int = 1`, plain
    /// `x = 1`, or augmented `x += 1` statement, measuring the left-hand
    /// side [paren-aware](Self::paren_aware). `None` for any other shape
    /// or when the span up to the `=` breaks across lines.
    fn qualify(&self, stmt: &Stmt) -> Option<aligner::Member> {
        match stmt {
            Stmt::AnnAssign(a) => {
                let value = a.value.as_deref()?;
                let annotation = self.paren_aware(a.annotation.as_ref().into(), a.into());
                self.equal_member(a.target.range().cover(annotation), value.start())
            }
            Stmt::Assign(a) => {
                let [target] = a.targets.as_slice() else {
                    return None;
                };
                self.equal_member(self.paren_aware(target.into(), a.into()), a.value.start())
            }
            Stmt::AugAssign(a) => {
                let target_range = self.paren_aware(a.target.as_ref().into(), a.into());
                aligner::range_anchored_member_single_line(
                    self.walker.source,
                    target_range,
                    TextRange::new(target_range.end(), a.value.start()),
                    |t| t.kind().as_augmented_assign_operator().is_some(),
                    a.op.as_str().len(),
                )
            }
            _ => None,
        }
    }

    /// Returns the alignment member for a `name=value` keyword argument,
    /// or `None` for a positional argument or a `**` unpacking. A keyword
    /// whose value spans lines still qualifies, since its `=` sits on the
    /// keyword's first line where [`Self::equal_member`] anchors it.
    fn qualify_keyword(&self, arg: ArgOrKeyword<'_>) -> Option<KeywordMember> {
        let ArgOrKeyword::Keyword(keyword) = arg else {
            return None;
        };
        let name = keyword.arg.as_ref()?;
        let member = self.equal_member(name.range(), keyword.value.start())?;
        let value_gap = TextRange::new(member.gap.end() + TextSize::of('='), keyword.value.start());
        Some(KeywordMember { member, value_gap })
    }

    /// Returns the alignment member for an annotated function parameter
    /// carrying a default value, or `None` for any other shape. Width
    /// spans the parameter name through the annotation's
    /// [paren-aware](Self::paren_aware) end, and the gap is the
    /// whitespace between that end and the `=` token.
    fn qualify_parameter(&self, param: AnyParameterRef<'_>) -> Option<aligner::Member> {
        let annotation = param.annotation()?;
        let default = param.default()?;
        let annotation_end = self
            .paren_aware(annotation.into(), param.as_parameter().into())
            .end();
        self.equal_member(
            TextRange::new(param.name().start(), annotation_end),
            default.start(),
        )
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

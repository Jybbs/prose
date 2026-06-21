//! Vertically aligns `=` across runs of same-indent, line-adjacent
//! assignments (single-target `Stmt::Assign`, `Stmt::AugAssign`,
//! initialized `Stmt::AnnAssign`), annotated parameter defaults, and an
//! exploded call's keyword arguments, aligning a run only when its rows
//! share a column baseline. Chained assignments, initializer-less
//! annotations, unannotated parameter defaults, and single-line
//! signatures or calls are skipped. Every aligned row reads as
//! `name = value`: the name side pads to the shared column, collapsing
//! to one space for a row that reaches no shared column (lone,
//! singleton, or differing-baseline), and the value side collapses to
//! one space after the operator. The value-side rewrite stops at the
//! value's parenthesis-inclusive start and sits out when the value
//! falls on a later line, so a wrapped or continued value keeps its
//! placement. A keyword condensed onto a line with another argument
//! keeps its tight `name=value`, and `+=` places `+` one column before
//! the shared `=`. Parameter widths reflect the post-`align_colons`
//! source.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, Parameters, Stmt,
    visitor::{Visitor as AstVisitor, walk_body, walk_expr, walk_stmt},
};
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    primitives::{
        aligner,
        equal_targets::{self, EqualMember},
    },
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
        if aligner::is_alignment_candidate(self.walker.source, &members) {
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
        let name_edits = if aligner::is_alignment_candidate(source, &members) {
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

    fn process_body(&mut self, body: &[Stmt]) {
        let source = self.walker.source;
        for group in aligner::line_adjacent_groups(source, body, self.walker.rule, |s| {
            equal_targets::assignment(source, s)
        }) {
            self.emit_equal_group(&group);
        }
    }

    /// Aligns each line-adjacent run of `call`'s keyword arguments that
    /// sit alone on their physical line, padding before each `=` and
    /// rewriting the gap after it to one space. A run pads its `=` only
    /// when its keywords share a column baseline. A lone keyword, or a
    /// run whose rows open at differing columns, instead takes a
    /// one-space buffer on each side of its `=`, so an exploded keyword
    /// reads as `name = value`. A keyword sharing its line with another
    /// argument keeps its tight `name=value`, and a single-line call or
    /// a held row is left untouched.
    fn process_call(&mut self, call: &ExprCall) {
        for group in equal_targets::keyword_groups(self.walker.source, self.walker.rule, call, true)
        {
            self.emit_equal_group(&group);
        }
    }

    /// Walks `params` through [`aligner::adjacent_member_groups`] with
    /// [`equal_targets::parameter`], emitting an alignment pass for each
    /// run of defaulted parameters. A multi-line default closes the run
    /// after it, so the parameters past it align as a separate group,
    /// mirroring an exploded call's keyword runs.
    fn process_parameters(&mut self, params: &Parameters) {
        let source = self.walker.source;
        let groups =
            aligner::adjacent_member_groups(source, params.iter_source_order(), true, |p| {
                equal_targets::parameter(source, p).into()
            });
        for group in groups {
            self.emit_aligned(&group);
        }
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

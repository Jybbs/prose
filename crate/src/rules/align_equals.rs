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
    visitor::{Visitor as AstVisitor, walk_body, walk_expr},
};

use crate::{
    config::Config,
    primitives::{aligner, equal_targets, walk::walk_stmt},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignEquals {
    settings: aligner::Settings,
}

impl AlignEquals {
    pub(crate) const MESSAGE: &'static str = "align consecutive `=` operators";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_equals)
                .with_line_length(config.code_width()),
        }
    }
}

impl Rule for AlignEquals {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            runs: Vec::new(),
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        let members = visitor.runs.iter().flat_map(|run| run.members()).copied();
        let widenings = aligner::Widenings::of(source, self.settings, members);
        visitor.walker.set_widenings(widenings);
        for run in std::mem::take(&mut visitor.runs) {
            match run {
                Run::Buffered(group) => visitor.walker.emit_group_or_buffer(&group),
                Run::Candidate(group) => visitor.walker.emit_if_candidate(&group),
            }
        }
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// One collected alignment run, held until every run is gathered so
/// the walker's widening entries cover the whole pass before any group
/// emits. `Buffered` runs take a column or each member's own buffer,
/// `Candidate` runs a column or nothing.
enum Run {
    Buffered(Vec<aligner::Member>),
    Candidate(Vec<aligner::Member>),
}

impl Run {
    fn members(&self) -> &[aligner::Member] {
        match self {
            Run::Buffered(group) | Run::Candidate(group) => group,
        }
    }
}

struct Visitor<'a> {
    runs: Vec<Run>,
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Aligns each adjacent assignment run in `body`, descending into
    /// every nested block the walk reaches.
    fn process_body(&mut self, body: &[Stmt]) {
        let source = self.walker.source;
        for group in aligner::line_adjacent_groups(source, body, self.walker.rule, |s| {
            equal_targets::assignment(source, s)
        }) {
            self.runs.push(Run::Buffered(group));
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
            self.runs.push(Run::Buffered(group));
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
            self.runs.push(Run::Candidate(aligner::retain_unheld(
                source,
                self.walker.rule,
                group,
            )));
        }
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

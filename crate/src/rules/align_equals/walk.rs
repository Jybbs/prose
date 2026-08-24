//! Walks the statement runs whose assignment operators share a column.

use ruff_python_ast::{
    Expr, ExprCall, Parameters, Stmt,
    visitor::{Visitor as AstVisitor, walk_body, walk_expr},
};

use super::*;

/// One collected alignment run, held until every run is gathered so
/// the walker's widening entries cover the whole pass before any group
/// emits. `Buffered` runs take a column or each member's own buffer,
/// `Candidate` runs a column or nothing.
pub(super) enum Run {
    Buffered(Vec<aligner::Member>),
    Candidate(Vec<aligner::Member>),
}

impl Run {
    pub(super) fn members(&self) -> &[aligner::Member] {
        match self {
            Run::Buffered(group) | Run::Candidate(group) => group,
        }
    }
}

pub(super) struct Visitor<'a> {
    pub(super) runs: Vec<Run>,
    pub(super) walker: aligner::AlignWalker<'a>,
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

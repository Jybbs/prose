//! The colon-context walk: the [`ColonEmitter`] contract every `:`
//! rule implements and the visitor that drives it across a module.
//! `contexts` builds the alignment member for each context and
//! `columns` settles the two columns a docstring entry run carries.

use ruff_python_ast::{
    Expr, Parameters, Stmt,
    visitor::{Visitor as AstVisitor, walk_body, walk_expr, walk_parameters, walk_stmt},
};

use crate::{
    primitives::{aligner, scope::scoped_body},
    rule::RuleId,
    source::Source,
};

mod columns;
mod contexts;

pub(crate) use columns::EntryColumns;
use columns::docstring_runs;
use contexts::{
    annotated_assignment_groups, dict_member_groups, match_case_members, parameter_groups,
};
pub(crate) use contexts::{match_case, match_case_pre_colon_end};

/// Receiver for the colon-context walker. `handle` is the catch-all
/// for annotated assignments, dict entries, and parameters, with
/// `match_arms` defaulting to it so a rule can override it.
/// `docstring_entries` carries both of a run's columns and defaults to
/// the `:` rows alone, leaving the type-group column to a rule that
/// overrides it. `rule` names the consuming
/// rule so the group builders can hold its skip-suppressed rows out of
/// alignment. Call `walk` to drive the emitter across `source`'s body.
pub(crate) trait ColonEmitter {
    fn docstring_entries(&mut self, run: &EntryColumns) {
        self.handle(run.colons());
    }

    fn handle(&mut self, members: &[aligner::Member]);

    fn match_arms(&mut self, members: &[aligner::Member]) {
        self.handle(members);
    }

    fn rule(&self) -> RuleId;

    /// Drives `self` across every `:` context in `source`'s module
    /// body. Recurses into nested classes, functions, matches, and
    /// expressions so a single call covers the whole tree.
    fn walk(&mut self, source: &Source)
    where
        Self: Sized,
    {
        let mut visitor = ContextVisitor {
            emitter: self,
            source,
        };
        visitor.consider_docstring(&source.ast().body);
        visitor.visit_body(&source.ast().body);
    }
}

struct ContextVisitor<'a, E> {
    emitter: &'a mut E,
    source: &'a Source,
}

impl<E: ColonEmitter> ContextVisitor<'_, E> {
    /// Emits every entry run in `body`'s leading docstring.
    fn consider_docstring(&mut self, body: &[Stmt]) {
        for run in docstring_runs(self.source, body) {
            self.emitter.docstring_entries(&run);
        }
    }
}

impl<'a, E: ColonEmitter> AstVisitor<'a> for ContextVisitor<'a, E> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        for group in annotated_assignment_groups(self.source, self.emitter.rule(), body) {
            self.emitter.handle(&group);
        }
        walk_body(self, body);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Dict(d) = expr {
            for group in dict_member_groups(self.source, self.emitter.rule(), d) {
                self.emitter.handle(&group);
            }
        }
        walk_expr(self, expr);
    }

    fn visit_parameters(&mut self, parameters: &'a Parameters) {
        for group in parameter_groups(self.source, self.emitter.rule(), parameters) {
            self.emitter.handle(&group);
        }
        walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Match(m) = stmt {
            self.emitter
                .match_arms(&match_case_members(self.source, &m.cases));
        }
        if let Some((body, _)) = scoped_body(stmt) {
            self.consider_docstring(body);
        }
        walk_stmt(self, stmt);
    }
}

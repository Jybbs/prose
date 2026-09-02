//! The colon-context walk: the [`ColonEmitter`] contract every `:`
//! rule implements and the visitor that drives it across a module.
//! `contexts` builds the alignment member for each context and
//! `columns` settles the two columns a docstring entry run carries,
//! reaching every docstring through the shared `walk_docstrings`.

use ruff_python_ast::{
    Expr, Parameters, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr, walk_parameters},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::{aligner, walk::walk_stmt},
    rule::RuleId,
    source::Source,
};

mod columns;
mod contexts;

pub(crate) use columns::EntryColumns;
use columns::docstring_runs_within;
use contexts::{
    annotated_assignment_groups, dict_member_groups, match_case_members, parameter_groups,
};
pub(crate) use contexts::{match_case, match_case_pre_colon_end};

/// Receiver for the colon-context walker. `handle` is the catch-all
/// for annotated assignments, dict entries, and parameters, with
/// `match_arms` defaulting to it so a rule can override it.
/// `docstring_entries` carries both of a run's columns and defaults to
/// the `:` rows alone, leaving the type-group column to a rule that
/// overrides it. `rule` names the consuming rule so the group builders
/// can hold its skip-suppressed rows out of alignment. Call `walk` to
/// drive the emitter across `source`.
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
        self.walk_within(source, &[source.module_range()]);
    }

    /// Drives `self` across every `:` context a statement reaching one
    /// of `windows` holds. Each body forms its groups across all of its
    /// siblings, since a group spans the body rather than a window, and
    /// the walk descends only into the statements a window reaches, so
    /// a context outside every window is visited only where its group
    /// straddles one.
    fn walk_within(&mut self, source: &Source, windows: &[TextRange])
    where
        Self: Sized,
    {
        for run in &docstring_runs_within(source, windows) {
            self.docstring_entries(run);
        }
        let mut visitor = ContextVisitor {
            emitter: self,
            source,
            windows,
        };
        visitor.visit_body(&source.ast().body);
    }
}

/// True where `range` sits wholly inside one of `windows`, ascending
/// and disjoint.
pub(crate) fn covers(range: TextRange, windows: &[TextRange]) -> bool {
    let at = windows.partition_point(|window| window.end() < range.end());
    windows
        .get(at)
        .is_some_and(|window| window.contains_range(range))
}

/// True where `range` overlaps one of `windows`, ascending and
/// disjoint, by more than a shared endpoint.
pub(crate) fn reaches(range: TextRange, windows: &[TextRange]) -> bool {
    let at = windows.partition_point(|window| window.end() <= range.start());
    windows
        .get(at)
        .is_some_and(|window| window.start() < range.end())
}

struct ContextVisitor<'a, E> {
    emitter: &'a mut E,
    source: &'a Source,
    windows: &'a [TextRange],
}

impl<'a, E: ColonEmitter> AstVisitor<'a> for ContextVisitor<'a, E> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        for group in annotated_assignment_groups(self.source, self.emitter.rule(), body) {
            self.emitter.handle(&group);
        }
        for stmt in body {
            if reaches(stmt.range(), self.windows) {
                self.visit_stmt(stmt);
            }
        }
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
        walk_stmt(self, stmt);
    }
}

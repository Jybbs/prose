//! Walks a module collecting the columns each reserved run settles to.

use rustc_hash::FxHashMap;

use super::*;
use crate::primitives::range::overlaps;

/// One collected alignment run, `candidate` where the rule aligns it
/// to a column or leaves it alone rather than buffering each row, and
/// `scope` the statement the run forms inside, the one whose body
/// holds a statement run or whose expressions hold a keyword or
/// parameter run, the module itself for a module-body run.
pub(super) struct Run {
    /// True for a run formed over a body's statements, false for one
    /// formed over a statement's keywords or parameters.
    pub(super) body: bool,
    pub(super) candidate: bool,
    pub(super) members: Vec<aligner::Member>,
    pub(super) scope: TextRange,
}

/// Collects the rule's runs and the value each member's row carries,
/// keyed by the paren-aware start the member's value gap ends at.
pub(super) struct ReserveVisitor<'a> {
    pub(super) rule: RuleId,
    pub(super) runs: Vec<Run>,
    pub(super) source: &'a Source,
    /// The statement the walk is inside, the scope a keyword or
    /// parameter run forms over, the module ahead of any.
    pub(super) stmt: TextRange,
    /// The completion of a carried table, the walk forming a body's
    /// runs over the slices it names and descending into the statements
    /// its windows reach alone, or `None` for a walk over the whole
    /// tree.
    pub(super) reform: Option<&'a Reform>,
    pub(super) values: FxHashMap<TextSize, (&'a Expr, AnyNodeRef<'a>)>,
}

impl<'a> ReserveVisitor<'a> {
    /// Records `value` under the start of its paren-aware range against
    /// `parent`.
    fn note(&mut self, value: &'a Expr, parent: AnyNodeRef<'a>) {
        let start = self.source.paren_aware_range(value.into(), parent).start();
        self.values.insert(start, (value, parent));
    }

    fn record(
        &mut self,
        groups: Vec<Vec<aligner::Member>>,
        candidate: bool,
        scope: TextRange,
        body: bool,
    ) {
        self.runs.extend(groups.into_iter().map(|members| Run {
            body,
            candidate,
            members,
            scope,
        }));
    }

    /// Notes the value of each assignment in `body` against its
    /// statement.
    fn note_values(&mut self, body: &'a [Stmt]) {
        for stmt in body {
            match stmt {
                Stmt::Assign(a) => self.note(&a.value, stmt.into()),
                Stmt::AugAssign(a) => self.note(&a.value, stmt.into()),
                Stmt::AnnAssign(a) => {
                    if let Some(value) = a.value.as_deref() {
                        self.note(value, stmt.into());
                    }
                }
                _ => {}
            }
        }
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let owner = self.stmt;
        let Some(reform) = self.reform else {
            self.record(
                equal_targets::assignment_groups(self.source, self.rule, body),
                false,
                owner,
                true,
            );
            self.note_values(body);
            for stmt in body {
                self.visit_stmt(stmt);
            }
            return;
        };
        for slice in reform.slices(owner, body) {
            self.record(
                equal_targets::assignment_groups(self.source, self.rule, &body[slice.clone()]),
                false,
                owner,
                true,
            );
            self.note_values(&body[slice]);
        }
        for stmt in body {
            if overlaps(stmt.range(), &reform.windows) {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.record(
                equal_targets::keyword_groups(self.source, self.rule, call, true),
                false,
                self.stmt,
                false,
            );
            for keyword in &call.arguments.keywords {
                self.note(&keyword.value, keyword.into());
            }
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let outer = std::mem::replace(&mut self.stmt, stmt.range());
        if let Stmt::FunctionDef(def) = stmt {
            self.record(
                equal_targets::parameter_groups(self.source, self.rule, &def.parameters),
                true,
                stmt.range(),
                false,
            );
            for param in def.parameters.iter_non_variadic_params() {
                if let Some(default) = param.default.as_deref() {
                    self.note(default, param.into());
                }
            }
        }
        walk::walk_stmt(self, stmt);
        self.stmt = outer;
    }
}

/// The widening the collected runs seat on each line under `settings`.
pub(super) fn widenings_over(
    source: &Source,
    settings: aligner::Settings,
    visitor: &ReserveVisitor,
) -> aligner::Widenings {
    aligner::Widenings::of(
        source,
        settings,
        visitor
            .runs
            .iter()
            .flat_map(|run| run.members.iter().copied()),
    )
}

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
    pub(super) values: FxHashMap<TextSize, (&'a Expr, AnyNodeRef<'a>)>,
    /// The spans a splice reparsed, the walk descending into the
    /// statements one reaches alone, or `None` for a walk over the
    /// whole tree.
    pub(super) windows: Option<&'a [TextRange]>,
}

impl<'a> ReserveVisitor<'a> {
    /// Records `value` under the start of its paren-aware range against
    /// `parent`.
    fn note(&mut self, value: &'a Expr, parent: AnyNodeRef<'a>) {
        let start = self.source.paren_aware_range(value.into(), parent).start();
        self.values.insert(start, (value, parent));
    }

    fn record(&mut self, groups: Vec<Vec<aligner::Member>>, candidate: bool, scope: TextRange) {
        self.runs.extend(groups.into_iter().map(|members| Run {
            candidate,
            members,
            scope,
        }));
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        self.record(
            equal_targets::assignment_groups(self.source, self.rule, body),
            false,
            self.stmt,
        );
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
        for stmt in body {
            if self
                .windows
                .is_none_or(|windows| overlaps(stmt.range(), windows))
            {
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

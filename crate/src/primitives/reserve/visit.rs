//! Walks a module collecting the columns each reserved run settles to.

use rustc_hash::FxHashMap;

use super::*;

/// One collected alignment run, `candidate` where the rule aligns it
/// to a column or leaves it alone rather than buffering each row.
pub(super) struct Run {
    pub(super) candidate: bool,
    pub(super) members: Vec<aligner::Member>,
}

/// Collects the rule's runs and the value each member's row carries,
/// keyed by the paren-aware start the member's value gap ends at.
pub(super) struct ReserveVisitor<'a> {
    pub(super) rule: RuleId,
    pub(super) runs: Vec<Run>,
    pub(super) source: &'a Source,
    pub(super) values: FxHashMap<TextSize, (&'a Expr, AnyNodeRef<'a>)>,
}

impl<'a> ReserveVisitor<'a> {
    /// Records `value` under the start of its paren-aware range against
    /// `parent`.
    fn note(&mut self, value: &'a Expr, parent: AnyNodeRef<'a>) {
        let start = self.source.paren_aware_range(value.into(), parent).start();
        self.values.insert(start, (value, parent));
    }

    fn record(&mut self, groups: Vec<Vec<aligner::Member>>, candidate: bool) {
        self.runs
            .extend(groups.into_iter().map(|members| Run { candidate, members }));
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    /// Builds the statement runs the way `align_equals` builds them, so
    /// a multi-line statement closes its run and a held one is
    /// transparent.
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let source = self.source;
        self.record(
            aligner::line_adjacent_groups(source, body, self.rule, |stmt| {
                equal_targets::assignment(source, stmt)
            }),
            false,
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
        walk_body(self, body);
    }

    /// Builds the keyword runs the way `align_equals` builds them, a
    /// multi-line value closing the run after it.
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.record(
                equal_targets::keyword_groups(self.source, self.rule, call, true),
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

    /// Builds the parameter-default runs the way `align_equals` builds
    /// them, a run aligning to a column or not at all and a multi-line
    /// default closing the run after it.
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(def) = stmt {
            let source = self.source;
            let groups = aligner::adjacent_member_groups(
                source,
                def.parameters.iter_source_order(),
                true,
                |param| equal_targets::parameter(source, param).into(),
            );
            let rule = self.rule;
            self.record(
                groups
                    .into_iter()
                    .map(|group| aligner::retain_unheld(source, rule, group))
                    .collect(),
                true,
            );
            for param in def.parameters.iter_non_variadic_params() {
                if let Some(default) = param.default.as_deref() {
                    self.note(default, param.into());
                }
            }
        }
        walk::walk_stmt(self, stmt);
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

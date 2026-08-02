//! Collapses a collection constructor wrapped around a form that
//! already builds that collection, and a comprehension that copies its
//! input unchanged. `set([x for x in xs])` reaches `set(xs)`, `dict()`
//! reaches `{}`, and `tuple([1])` reaches `(1,)`. An empty sequence
//! reaches `set()` rather than a brace form, because `{}` names an empty
//! dict. A constructor name the module binds itself leaves every call to
//! it alone.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    rule::{Rule, RuleId},
    source::Source,
};

mod constructor;
mod plan;
mod render;

use self::{constructor::Constructor, plan::plan_for, render::edits_for};

pub(crate) struct SimplifyComprehensions;

impl SimplifyComprehensions {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for SimplifyComprehensions {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let analysis = source.binding_analysis();
        let rebound: Vec<Constructor> = Constructor::ALL
            .into_iter()
            .filter(|ctor| analysis.binds_name(ctor.as_str()))
            .collect();
        let mut walker = Walker {
            groups: Vec::new(),
            rebound: &rebound,
            source,
        };
        walker.visit_body(&source.ast().body);
        walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Visits every expression in source order, recording the rewrite each
/// earns. An enclosing rewrite lands first, so a nested candidate whose
/// edits overlap it drops.
struct Walker<'a> {
    groups: Vec<Vec<Edit>>,
    rebound: &'a [Constructor],
    source: &'a Source,
}

impl Walker<'_> {
    /// True when a recorded edit covers any of `range`.
    fn claimed(&self, range: TextRange) -> bool {
        self.groups
            .iter()
            .flatten()
            .any(|edit| edit.start() < range.end() && range.start() < edit.end())
    }

    /// Records the rewrite `expr` earns, unless one of its edits falls
    /// where an already-recorded edit does.
    fn record(&mut self, expr: &Expr) {
        let Some(edits) =
            plan_for(expr, self.rebound).and_then(|plan| edits_for(self.source, expr, &plan))
        else {
            return;
        };
        if !edits.iter().any(|edit| self.claimed(edit.range())) {
            self.groups.push(edits);
        }
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.record(expr);
        walk_expr(self, expr);
    }
}

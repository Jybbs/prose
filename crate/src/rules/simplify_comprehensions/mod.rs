//! Collapses a collection constructor wrapped around a form that
//! already builds that collection, and a comprehension that copies its
//! input unchanged. `set([x for x in xs])` reaches `set(xs)`, `dict()`
//! reaches `{}`, and `tuple([1])` reaches `(1,)`. An empty sequence
//! reaches `set()` rather than a brace form, because `{}` names an empty
//! dict. A constructor name the module binds itself leaves every call to
//! it alone. The walk reads expressions in source order, so an enclosing
//! rewrite is recorded first and a nested candidate whose edits fall
//! where it already edits drops.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::walk::{Descent, filter_map_over_exprs},
    rule::{Rule, RuleId},
    source::Source,
};

mod constructor;
mod plan;
mod render;

use self::{constructor::Constructor, plan::plan_for, render::edits_for};

#[derive(Debug)]
pub(crate) struct SimplifyComprehensions;

impl SimplifyComprehensions {
    pub(crate) const MESSAGE: &'static str = "collapse a redundant collection constructor";

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
        let mut claimed: Vec<TextRange> = Vec::new();
        filter_map_over_exprs(&source.ast().body, Descent::Over, |expr| {
            let plan = plan_for(expr, &rebound)?;
            let edits = edits_for(source, expr, &plan)?;
            if edits.iter().any(|edit| overlaps(&claimed, edit.range())) {
                return None;
            }
            claimed.extend(edits.iter().map(Ranged::range));
            Some(edits)
        })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// True when `range` shares at least one byte with a recorded range.
fn overlaps(claimed: &[TextRange], range: TextRange) -> bool {
    claimed.iter().any(|held| held.ordering(range).is_eq())
}

//! Topological tiering of definition runs by evaluation-time
//! dependency, alongside the soundness check a reorder runs against
//! that same reference graph and the fence a class raises where its
//! base list runs a hook no static read of the module follows.

use itertools::Itertools;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

mod fences;
mod reach;
mod refs;
mod runs;
mod strands;
mod tiers;

pub(crate) use fences::fenced_slots;
pub(crate) use reach::{CallReach, call_reachable};
use reach::{called_names, calls_a_name};
use refs::eval_time_refs;
pub(crate) use refs::{eval_refs, observed_refs, walk_lambda_defaults};
pub(crate) use runs::{DefRun, def_run_tier_keys};
pub(crate) use strands::Strands;
pub(crate) use tiers::tier_levels;

/// The evaluation-time references and the evaluated names of a body,
/// the pair an [`Evaluation`] borrows.
pub(crate) struct Evaluated<'src> {
    names: FxHashMap<TextSize, Vec<&'src str>>,
    refs: FxHashMap<TextSize, Vec<&'src str>>,
}

impl<'src> Evaluated<'src> {
    /// The pair over `body`, widening through `reachable`. `refs` is the
    /// evaluation-time reference map [`eval_time_refs_of`] builds, taken
    /// prebuilt so a caller reading it for its own gate walks the body
    /// once rather than twice.
    pub(crate) fn of(
        body: &'src [Stmt],
        reachable: &CallReach<'src>,
        refs: FxHashMap<TextSize, Vec<&'src str>>,
    ) -> Self {
        Self {
            names: evaluated_names_of(body, reachable, &refs),
            refs,
        }
    }

    pub(crate) fn evaluation(&self) -> Evaluation<'_, 'src> {
        Evaluation {
            names: &self.names,
            refs: &self.refs,
        }
    }
}

/// What evaluating a statement reads, being the module-scope names it
/// evaluates and the evaluation-time references of every statement in
/// the body, each keyed by start offset.
#[derive(Clone, Copy)]
pub(crate) struct Evaluation<'a, 'src> {
    pub(crate) names: &'a FxHashMap<TextSize, Vec<&'src str>>,
    pub(crate) refs: &'a FxHashMap<TextSize, Vec<&'src str>>,
}

impl<'a, 'src> Evaluation<'a, 'src> {
    /// Every module-scope name evaluating `stmt` reads, empty for a
    /// statement outside the body the cache was built over.
    fn names(self, stmt: &Stmt) -> &'a [&'src str] {
        lookup(self.names, stmt)
    }

    /// The evaluation-time references of `stmt`, empty for a statement
    /// outside the body the cache was built over.
    fn refs_of(self, stmt: &Stmt) -> &'a [&'src str] {
        lookup(self.refs, stmt)
    }
}

/// The names `map` holds for `stmt`, empty for a statement outside the
/// body the map was built over.
fn lookup<'a, 'src>(map: &'a FxHashMap<TextSize, Vec<&'src str>>, stmt: &Stmt) -> &'a [&'src str] {
    map.get(&stmt.start()).map_or(&[], Vec::as_slice)
}

/// True where some statement of `body` reaches another definition's body
/// at evaluation time, so the run needs the call graph. A non-definition
/// reaches one by calling a name, and a definition reaches one where its
/// own evaluation surface, a base list or a decorator or a default, names
/// a sibling definition, read off the `refs` map rather than walked again.
pub(crate) fn consults_call_graph(
    body: &[Stmt],
    refs: &FxHashMap<TextSize, Vec<&str>>,
    defined: &FxHashSet<&str>,
) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::FunctionDef(_) | Stmt::ClassDef(_) => refs
            .get(&stmt.start())
            .is_some_and(|names| names.iter().any(|name| defined.contains(name))),
        _ => calls_a_name(stmt),
    })
}

/// Every name a module-level definition of `body` binds.
pub(crate) fn definition_names(body: &[Stmt]) -> FxHashSet<&str> {
    body.iter().filter_map(definition_name).collect()
}

/// The evaluation-time references of every statement in `body`, keyed
/// by start offset, each list holding a name once.
pub(crate) fn eval_time_refs_of(
    body: &[Stmt],
    defer_annotations: bool,
) -> FxHashMap<TextSize, Vec<&str>> {
    body.iter()
        .map(|stmt| {
            let refs = eval_time_refs(stmt, defer_annotations)
                .into_iter()
                .unique()
                .collect();
            (stmt.start(), refs)
        })
        .collect()
}

/// The name a module-level definition binds, `None` for any other
/// statement.
fn definition_name(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::ClassDef(class) => Some(class.name.as_str()),
        Stmt::FunctionDef(func) => Some(func.name.as_str()),
        _ => None,
    }
}

/// Every module-scope name evaluating each statement of `body` reads,
/// keyed by start offset: its own evaluation-time references, widened
/// by the reach of every definition those references name where the
/// statement is a definition and by the reach of every definition it
/// calls otherwise.
fn evaluated_names_of<'src>(
    body: &'src [Stmt],
    reachable: &CallReach<'src>,
    refs: &FxHashMap<TextSize, Vec<&'src str>>,
) -> FxHashMap<TextSize, Vec<&'src str>> {
    body.iter()
        .map(|stmt| {
            let own = lookup(refs, stmt);
            let called;
            let runs: &[&str] = if reachable.is_empty()
                || matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_))
            {
                own
            } else {
                called = called_names(stmt);
                &called
            };
            let names = own
                .iter()
                .copied()
                .chain(
                    runs.iter()
                        .flat_map(|ran| reachable.get(ran))
                        .flatten()
                        .copied(),
                )
                .unique()
                .collect();
            (stmt.start(), names)
        })
        .collect()
}

//! Topological tiering of definition runs by evaluation-time
//! dependency, so a definition never sorts ahead of a sibling it names
//! at evaluation time. Shared by the module-constant banding in
//! `super::bands` and the body rewriter's def-run reorder in `super`.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{
    Expr, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr, walk_parameters, walk_stmt},
};
use ruff_text_size::{Ranged, TextSize};

use crate::primitives::orderer::permute_full;

/// Accumulates load-context names through `eval_time_refs`, pruning
/// function and lambda bodies and skipping deferred annotations.
struct EvalRefVisitor<'src> {
    defer_annotations: bool,
    names: Vec<&'src str>,
}

impl<'src> AstVisitor<'src> for EvalRefVisitor<'src> {
    fn visit_annotation(&mut self, annotation: &'src Expr) {
        if !self.defer_annotations {
            self.visit_expr(annotation);
        }
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        match expr {
            Expr::Lambda(lambda) => {
                if let Some(params) = lambda.parameters.as_deref() {
                    walk_parameters(self, params);
                }
            }
            Expr::Name(name) if name.ctx.is_load() => self.names.push(name.id.as_str()),
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::AnnAssign(ann) => {
                self.visit_annotation(&ann.annotation);
                if let Some(value) = &ann.value {
                    self.visit_expr(value);
                }
            }
            Stmt::FunctionDef(func) => {
                for decorator in &func.decorator_list {
                    self.visit_expr(&decorator.expression);
                }
                walk_parameters(self, &func.parameters);
                if let Some(returns) = &func.returns {
                    self.visit_annotation(returns);
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

/// Collects the load-context names in `expr`, pruning every function
/// and lambda body, the reference set a module constant's value or
/// annotation contributes to the hoist graph.
pub(super) fn eval_refs(expr: &Expr) -> Vec<&str> {
    let mut visitor = EvalRefVisitor {
        defer_annotations: true,
        names: Vec::new(),
    };
    visitor.visit_expr(expr);
    visitor.names
}

/// Collects the load-context names in a definition's evaluation-time
/// surface: its decorators, base classes and class keywords, parameter
/// defaults, non-deferred annotations, and the top level of a class
/// body, descending into nested definitions but pruning every function
/// and lambda body. Annotation positions are skipped when
/// `defer_annotations` holds.
pub(super) fn eval_time_refs(stmt: &Stmt, defer_annotations: bool) -> Vec<&str> {
    let mut visitor = EvalRefVisitor {
        defer_annotations,
        names: Vec::new(),
    };
    visitor.visit_stmt(stmt);
    visitor.names
}

/// Tiers the definition run selected by `member` and permutes `order`
/// by `(tier, key)`, leaving `order` untouched when the run declines.
pub(super) fn permute_defs<'src, K: Copy + Ord>(
    order: &mut [usize],
    body: &'src [Stmt],
    defer_annotations: bool,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
) {
    if let Some(keys) = def_run_tier_keys(body, defer_annotations, member) {
        permute_full(order, body, |s| keys.get(&s.range().start()).copied());
    }
}

/// Assigns each binding a Kahn-style topological tier from its
/// intra-run dependency set. Tier 0 is bindings with no deps, tier N
/// is bindings whose deps all sit in tiers strictly less than N.
/// Returns `None` when any binding remains untiered after `n`
/// iterations.
pub(super) fn tier_levels(dep_sets: &[HashSet<usize>]) -> Option<Vec<usize>> {
    let n = dep_sets.len();
    let mut tiers: Vec<Option<usize>> = vec![None; n];
    for _ in 0..n {
        for i in 0..n {
            if tiers[i].is_some() || !dep_sets[i].iter().all(|&d| tiers[d].is_some()) {
                continue;
            }
            tiers[i] = Some(
                dep_sets[i]
                    .iter()
                    .filter_map(|&d| tiers[d])
                    .max()
                    .map_or(0, |t| t + 1),
            );
        }
    }
    tiers.into_iter().collect()
}

/// Returns a per-member `(tier, key)` lookup keyed by each definition's
/// start offset, or `None` when the run cannot reorder. The run skips
/// when two members share a name or when the intra-run reference graph
/// carries a cycle. A member depends on every other sibling it names in
/// its evaluation-time surface, and the composite `(tier, key)` combines
/// a Kahn-style topological tier with the member's existing sort key, so
/// a definition never sorts ahead of a sibling it names at evaluation
/// time.
pub(super) fn def_run_tier_keys<'src, K: Copy>(
    body: &'src [Stmt],
    defer_annotations: bool,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
) -> Option<HashMap<TextSize, (usize, K)>> {
    let members: Vec<(&'src Stmt, &'src str, K)> = body
        .iter()
        .filter_map(|stmt| member(stmt).map(|(name, key)| (stmt, name, key)))
        .collect();
    let name_to_idx = unique_name_index(members.iter().map(|&(_, name, _)| name))?;
    let dep_sets: Vec<HashSet<usize>> = members
        .iter()
        .enumerate()
        .map(|(idx, &(stmt, _, _))| {
            eval_time_refs(stmt, defer_annotations)
                .into_iter()
                .filter_map(|name| name_to_idx.get(name).copied())
                // A recursive self-reference does not constrain sibling order.
                .filter(|&dep| dep != idx)
                .collect()
        })
        .collect();
    tier_key_map(
        members
            .into_iter()
            .map(|(stmt, _, key)| (stmt.range().start(), key)),
        &dep_sets,
    )
}

/// Tiers `dep_sets` and assembles a per-statement `(tier, key)` lookup
/// keyed by start offset, or `None` when the dependency graph cycles.
/// `offsets_keys` must yield one `(offset, key)` pair per dep set, in
/// order.
fn tier_key_map<K>(
    offsets_keys: impl Iterator<Item = (TextSize, K)>,
    dep_sets: &[HashSet<usize>],
) -> Option<HashMap<TextSize, (usize, K)>> {
    let tiers = tier_levels(dep_sets)?;
    Some(
        offsets_keys
            .zip(tiers)
            .map(|((offset, key), tier)| (offset, (tier, key)))
            .collect(),
    )
}

/// Indexes each name to its position, or `None` when a name repeats. A
/// duplicate makes an intra-run reference ambiguous, so the caller
/// declines the reorder.
fn unique_name_index<'a>(names: impl Iterator<Item = &'a str>) -> Option<HashMap<&'a str, usize>> {
    let mut index = HashMap::new();
    for (position, name) in names.enumerate() {
        if index.insert(name, position).is_some() {
            return None;
        }
    }
    Some(index)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    fn class_member(stmt: &Stmt) -> Option<(&str, &str)> {
        stmt.as_class_def_stmt().map(|class| {
            let name = class.name.as_str();
            (name, name)
        })
    }

    #[test]
    fn def_run_tier_keys_declines_a_duplicate_member_name() {
        let source = parse("class Dup:\n    pass\n\n\nclass Dup:\n    pass\n");
        assert!(def_run_tier_keys(&source.ast().body, false, class_member).is_none());
    }

    #[test]
    fn def_run_tier_keys_declines_a_reference_cycle() {
        let source = parse("class Alpha(Beta):\n    pass\n\n\nclass Beta(Alpha):\n    pass\n");
        assert!(def_run_tier_keys(&source.ast().body, false, class_member).is_none());
    }

    #[test]
    fn def_run_tier_keys_excludes_a_recursive_self_reference() {
        let source = parse("class Node:\n    def child(self) -> Node: ...\n");
        let body = &source.ast().body;
        let keys =
            def_run_tier_keys(body, false, class_member).expect("self-reference does not decline");
        assert_eq!(keys[&body[0].range().start()].0, 0);
    }

    #[test]
    fn def_run_tier_keys_tiers_a_backward_base_class_reference() {
        let source = parse("class Beta:\n    pass\n\n\nclass Alpha(Beta):\n    pass\n");
        let body = &source.ast().body;
        let keys = def_run_tier_keys(body, false, class_member).expect("acyclic run tiers");
        let tier = |i: usize| keys[&body[i].range().start()].0;
        assert_eq!(tier(0), 0, "Beta has no dependency");
        assert_eq!(tier(1), 1, "Alpha depends on Beta");
    }

    #[test]
    fn eval_time_refs_collects_eval_surface_and_skips_bodies() {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef:
                    return BodyRef
        "});
        let collected: HashSet<&str> = eval_time_refs(&source.ast().body[0], false)
            .into_iter()
            .collect();
        assert_eq!(
            collected,
            HashSet::from(["AnnotRef", "BaseRef", "DefaultRef", "ParamRef", "ReturnRef"]),
        );
    }

    #[test]
    fn eval_time_refs_prunes_a_lambda_body() {
        let source = parse("class Probe:\n    factory = lambda seed=SeedRef: BodyRef\n");
        let collected: HashSet<&str> = eval_time_refs(&source.ast().body[0], false)
            .into_iter()
            .collect();
        assert_eq!(collected, HashSet::from(["SeedRef"]));
    }

    #[test]
    fn eval_time_refs_skips_annotations_when_deferred() {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef: ...
        "});
        let collected: HashSet<&str> = eval_time_refs(&source.ast().body[0], true)
            .into_iter()
            .collect();
        assert_eq!(collected, HashSet::from(["BaseRef", "DefaultRef"]));
    }

    #[test]
    fn tier_levels_assigns_zero_for_empty_deps() {
        let deps = vec![HashSet::new(), HashSet::new(), HashSet::new()];
        assert_eq!(tier_levels(&deps), Some(vec![0, 0, 0]));
    }

    #[test]
    fn tier_levels_climbs_through_chain() {
        let deps = vec![
            HashSet::new(),
            HashSet::from([0]),
            HashSet::from([1]),
            HashSet::from([0, 2]),
        ];
        assert_eq!(tier_levels(&deps), Some(vec![0, 1, 2, 3]));
    }

    #[rstest]
    #[case(vec![HashSet::from([0])])]
    #[case(vec![HashSet::from([1]), HashSet::from([0])])]
    #[case(vec![HashSet::from([1]), HashSet::from([2]), HashSet::from([0])])]
    fn tier_levels_returns_none_on_cycles(#[case] deps: Vec<HashSet<usize>>) {
        assert_eq!(tier_levels(&deps), None);
    }

    proptest! {
        #[test]
        fn tier_levels_assigns_dependency_respecting_tiers_for_dags(
            deps in prop::collection::vec(prop::collection::vec(0usize..16, 0..4), 1..16),
        ) {
            let dag: Vec<HashSet<usize>> = deps
                .into_iter()
                .enumerate()
                .map(|(i, ds)| ds.into_iter().filter(|&d| d < i).collect())
                .collect();
            let Some(tiers) = tier_levels(&dag) else {
                return Err(TestCaseError::fail("acyclic input must tier"));
            };
            for (i, ds) in dag.iter().enumerate() {
                for &d in ds {
                    prop_assert!(
                        tiers[i] > tiers[d],
                        "binding {i} (tier {}) must sit strictly above dep {d} (tier {})",
                        tiers[i],
                        tiers[d],
                    );
                }
            }
        }

        #[test]
        fn tier_levels_rejects_inputs_with_self_loops(
            n in 1usize..8,
            cycle_index in 0usize..8,
        ) {
            let cycle_index = cycle_index.min(n - 1);
            let mut deps: Vec<HashSet<usize>> = (0..n).map(|_| HashSet::new()).collect();
            deps[cycle_index].insert(cycle_index);
            prop_assert_eq!(tier_levels(&deps), None);
        }
    }
}

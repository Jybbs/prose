//! Topological tiering of definition runs by evaluation-time
//! dependency, alongside the soundness check a reorder runs against
//! that same reference graph.

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    ops::Range,
};

mod refs;

/// Per module-level definition, the module-scope names a call into it
/// can reach.
pub(crate) type CallReach<'src> = HashMap<&'src str, HashSet<&'src str>>;

pub(crate) use refs::{eval_refs, eval_time_refs, observed_refs, walk_lambda_defaults};

use ruff_python_ast::{
    ExceptHandler, Expr, ExprContext, ExprLambda, Parameter, Stmt,
    visitor::{self, Visitor as AstVisitor, walk_expr, walk_parameters},
};
use ruff_text_size::{Ranged, TextSize};

use crate::primitives::{
    binding::{bare_import_bound_name, from_import_bound_name, module_bound_names},
    orderer::permute_in_place,
    slots::slot_positions,
    walk::walk_stmt,
};

/// Returns a per-member `(tier, key)` lookup keyed by each definition's
/// start offset, or `None` when the run cannot reorder. The run skips
/// when two members share a name or when the intra-run reference graph
/// carries a cycle. A member depends on every other sibling it names in
/// its evaluation-time surface, and the composite `(tier, key)` combines
/// a Kahn-style topological tier with the member's existing sort key, so
/// a definition never sorts ahead of a sibling it names at evaluation
/// time.
pub(crate) fn def_run_tier_keys<'src, K: Copy>(
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

/// Tiers the `member`-selected definitions within `range` and permutes
/// those slots of `order` by `(tier, key)`, leaving `order` untouched
/// when the run declines. A member `holds` selects keeps its source
/// slot, and the permutation reverts when it seats a definition below a
/// statement that names it.
pub(crate) fn permute_defs<'src, K: Copy + Ord>(
    order: &mut [usize],
    body: &'src [Stmt],
    range: Range<usize>,
    defer_annotations: bool,
    reachable: &CallReach<'src>,
    holds: impl Fn(&'src Stmt) -> bool,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
) {
    let Some(keys) = def_run_tier_keys(&body[range.clone()], defer_annotations, &member) else {
        return;
    };
    permute_or_revert(
        order,
        body,
        &range,
        defer_annotations,
        reachable,
        |stmt| member(stmt).map(|(name, _)| name),
        |order| {
            permute_in_place(order, body, range.clone(), |stmt| {
                keys.get(&stmt.range().start())
                    .copied()
                    .filter(|_| !holds(stmt))
            })
        },
    );
}

/// Runs `permute` against `order`, restoring the pre-permutation slots
/// when it moves a slot and the result seats a `member_name` entry below
/// a statement that names it.
pub(crate) fn permute_or_revert<'src>(
    order: &mut [usize],
    body: &'src [Stmt],
    range: &Range<usize>,
    defer_annotations: bool,
    reachable: &CallReach<'src>,
    member_name: impl Fn(&'src Stmt) -> Option<&'src str>,
    permute: impl FnOnce(&mut [usize]) -> bool,
) {
    let snapshot = order.to_vec();
    if permute(order)
        && !order_keeps_refs_backward(
            order,
            body,
            range,
            defer_annotations,
            member_name,
            reachable,
        )
    {
        order.copy_from_slice(&snapshot);
    }
}

/// Assigns each binding a Kahn-style topological tier from its
/// intra-run dependency set. Tier 0 is bindings with no deps, tier N
/// is bindings whose deps all sit in tiers strictly less than N.
/// Returns `None` when any binding remains untiered after `n`
/// iterations.
pub(crate) fn tier_levels(dep_sets: &[HashSet<usize>]) -> Option<Vec<usize>> {
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

/// True when every statement in `range` stays on the side of each
/// binding it evaluates that the source seated it, covering both the
/// members `member_name` selects and the non-member bindings among
/// them, so a reader neither rises above the binding that introduces a
/// name nor crosses a rebinding the source placed after it. A statement
/// naming itself imposes nothing.
fn order_keeps_refs_backward<'src>(
    order: &[usize],
    body: &'src [Stmt],
    range: &Range<usize>,
    defer_annotations: bool,
    member_name: impl Fn(&'src Stmt) -> Option<&'src str>,
    reachable: &CallReach<'src>,
) -> bool {
    let member_at: HashMap<&'src str, usize> = body[range.clone()]
        .iter()
        .zip(range.clone())
        .filter_map(|(stmt, at)| member_name(stmt).map(|name| (name, at)))
        .collect();
    let mut bound_at: HashMap<&'src str, Vec<usize>> = HashMap::new();
    for (stmt, at) in body[range.clone()]
        .iter()
        .zip(range.clone())
        .filter(|(stmt, _)| member_name(stmt).is_none())
    {
        for name in module_bound_names(stmt) {
            bound_at.entry(name).or_default().push(at);
        }
    }
    let position = slot_positions(order);
    body[range.clone()]
        .iter()
        .zip(range.clone())
        .all(|(stmt, reader)| {
            let mut names = eval_time_refs(stmt, defer_annotations);
            if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
                for called in called_names(stmt) {
                    if let Some(reached) = reachable.get(called) {
                        names.extend(reached.iter().copied());
                    }
                }
            }
            names.into_iter().all(|name| {
                member_at
                    .get(name)
                    .is_none_or(|&referent| side_kept(referent, reader, &position))
                    && bound_at.get(name).is_none_or(|binders| {
                        binders
                            .iter()
                            .all(|&binder| side_kept(binder, reader, &position))
                    })
            })
        })
}

/// Collects the names one definition reads and the names it binds.
struct ScopeScan<'src> {
    binds: HashSet<&'src str>,
    reads: HashSet<&'src str>,
}

impl<'src> AstVisitor<'src> for ScopeScan<'src> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        if let Expr::Name(name) = expr {
            if matches!(name.ctx, ExprContext::Load) {
                self.reads.insert(name.id.as_str());
            } else {
                self.binds.insert(name.id.as_str());
            }
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::FunctionDef(func) => {
                self.binds.insert(func.name.as_str());
            }
            Stmt::ClassDef(class) => {
                self.binds.insert(class.name.as_str());
            }
            Stmt::Import(import) => self
                .binds
                .extend(import.names.iter().map(bare_import_bound_name)),
            Stmt::ImportFrom(import) => self
                .binds
                .extend(import.names.iter().map(from_import_bound_name)),
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_parameter(&mut self, parameter: &'src Parameter) {
        self.binds.insert(parameter.name.as_str());
        visitor::walk_parameter(self, parameter);
    }

    fn visit_except_handler(&mut self, handler: &'src ExceptHandler) {
        let ExceptHandler::ExceptHandler(caught) = handler;
        if let Some(name) = &caught.name {
            self.binds.insert(name.as_str());
        }
        visitor::walk_except_handler(self, handler);
    }
}

/// The names a call into `stmt` evaluates that `stmt` does not bind
/// itself, being the module-scope surface its body reads.
fn free_reads(stmt: &Stmt) -> HashSet<&str> {
    let mut scan = ScopeScan {
        binds: HashSet::new(),
        reads: HashSet::new(),
    };
    scan.visit_stmt(stmt);
    scan.reads.retain(|name| !scan.binds.contains(name));
    scan.reads
}

/// Every name `stmt` calls directly.
pub(crate) fn called_names(stmt: &Stmt) -> Vec<&str> {
    struct Calls<'src>(Vec<&'src str>);
    impl<'src> AstVisitor<'src> for Calls<'src> {
        fn visit_expr(&mut self, expr: &'src Expr) {
            if let Expr::Call(call) = expr
                && let Expr::Name(name) = call.func.as_ref()
            {
                self.0.push(name.id.as_str());
            }
            walk_expr(self, expr);
        }
    }
    let mut calls = Calls(Vec::new());
    calls.visit_stmt(stmt);
    calls.0
}

/// Per module-level definition, every module-scope name a call into it
/// can reach, following call edges between definitions to a fixed point.
/// A class contributes every name its whole body reads, because a
/// constructor reaches its siblings through `self` attribute calls the
/// call graph cannot follow and an attribute access runs `__getattr__`
/// with no call at all.
pub(crate) fn call_reachable(body: &[Stmt]) -> CallReach<'_> {
    let mut reads: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(func) => {
                reads.insert(func.name.as_str(), free_reads(stmt));
                edges.insert(func.name.as_str(), called_names(stmt));
            }
            Stmt::ClassDef(class) => {
                reads.insert(class.name.as_str(), free_reads(stmt));
                edges.insert(class.name.as_str(), called_names(stmt));
            }
            _ => continue,
        }
    }
    for edge in edges.values_mut() {
        edge.retain(|callee| reads.contains_key(callee));
    }
    loop {
        let mut pending: Vec<(&str, HashSet<&str>)> = Vec::new();
        for (&name, callees) in &edges {
            let mut extra: HashSet<&str> = HashSet::new();
            for callee in callees.iter().filter(|callee| **callee != name) {
                for reached in &reads[callee] {
                    if !reads[name].contains(reached) {
                        extra.insert(reached);
                    }
                }
            }
            if !extra.is_empty() {
                pending.push((name, extra));
            }
        }
        if pending.is_empty() {
            break;
        }
        for (name, extra) in pending {
            reads.get_mut(name).unwrap().extend(extra);
        }
    }
    reads
}

/// True when `binding` stays on the side of `reader` that the source
/// seated it, so a reader neither rises above a binding it evaluates
/// nor crosses one the source placed after it. A statement binding the
/// name it reads imposes nothing on itself.
fn side_kept(binding: usize, reader: usize, position: &[usize]) -> bool {
    match binding.cmp(&reader) {
        Ordering::Less => position[binding] < position[reader],
        Ordering::Greater => position[binding] > position[reader],
        Ordering::Equal => true,
    }
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
    use crate::{primitives::decorator::is_decorated, testing::parse};

    fn class_member(stmt: &Stmt) -> Option<(&str, &str)> {
        stmt.as_class_def_stmt().map(|class| {
            let name = class.name.as_str();
            (name, name)
        })
    }

    /// The new-order permutation `permute_defs` produces over `src`'s
    /// class run, holding each member `holds` selects.
    fn class_order(src: &str, holds: fn(&Stmt) -> bool) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let reachable = call_reachable(body);
        permute_defs(
            &mut order,
            body,
            0..body.len(),
            false,
            &reachable,
            holds,
            class_member,
        );
        order
    }

    /// The new-order permutation `permute_defs` produces over `src`'s
    /// function run, holding nothing.
    fn func_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let reachable = call_reachable(body);
        permute_defs(
            &mut order,
            body,
            0..body.len(),
            false,
            &reachable,
            |_| false,
            |stmt| {
                stmt.as_function_def_stmt().map(|func| {
                    let name = func.name.as_str();
                    (name, name)
                })
            },
        );
        order
    }

    #[test]
    fn call_reachable_covers_a_name_a_constructor_reaches_through_self() {
        let source = parse(indoc! {"
            def zzz_helper():
                return 1

            class C:
                def __init__(self):
                    self.value = self.compute()

                def compute(self):
                    return zzz_helper()
        "});
        let reachable = call_reachable(&source.ast().body);
        assert!(
            reachable["C"].contains("zzz_helper"),
            "instantiating C runs compute through a self call, evaluating zzz_helper"
        );
    }

    #[test]
    fn permute_defs_holds_a_definition_a_constructor_reaches_through_self() {
        let src = indoc! {"
            def zzz_helper():
                return 1

            class C:
                def __init__(self):
                    self.value = self.compute()

                def compute(self):
                    return zzz_helper()

            obj = C()

            def aaa():
                pass
        "};
        assert_eq!(
            func_order(src),
            vec![0, 1, 2, 3],
            "the instantiation reaches zzz_helper, so the run holds its source order"
        );
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
    fn permute_defs_exempts_a_held_member_naming_itself() {
        let src = indoc! {"
            class Mid:
                pass

            @dec
            class Node:
                child: Node

            class Alpha:
                pass
        "};
        assert_eq!(
            class_order(src, is_decorated),
            vec![2, 1, 0],
            "Node names only itself, so the hold strands nothing"
        );
    }

    #[test]
    fn permute_defs_holds_a_decorated_definition() {
        let src = indoc! {"
            class Zeta:
                pass

            @dec
            class Alpha:
                pass

            class Mid:
                pass
        "};
        assert_eq!(
            class_order(src, is_decorated),
            vec![2, 1, 0],
            "Alpha holds slot 1 while Zeta and Mid swap around it"
        );
        assert_eq!(
            class_order(src, |_| false),
            vec![1, 2, 0],
            "without the hold Alpha sorts to the front"
        );
    }

    #[test]
    fn permute_defs_reverts_when_a_hold_strands_a_base_class() {
        let src = indoc! {"
            class Mid:
                pass

            @dec
            class Zeta(Mid):
                pass

            class Alpha:
                pass
        "};
        assert_eq!(
            class_order(src, is_decorated),
            vec![0, 1, 2],
            "Zeta holds its slot, so Mid may not sort below it"
        );
        assert_eq!(
            class_order(src, |_| false),
            vec![2, 0, 1],
            "without the hold the tier graph seats Mid ahead of Zeta"
        );
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
    #[rstest]
    #[case::nested_class(
        "def outer():\n    class Local(Outside):\n        pass\n    return Local\n",
        "Local"
    )]
    #[case::bare_import("def outer():\n    import osmod\n    return osmod, Outside\n", "osmod")]
    #[case::from_import(
        "def outer():\n    from pkg import thing\n    return thing, Outside\n",
        "thing"
    )]
    #[case::except_alias(
        "def outer():\n    try:\n        Outside()\n    except ValueError as err:\n        return err\n",
        "err"
    )]
    fn free_reads_excludes_a_name_the_body_binds(#[case] src: &str, #[case] bound: &str) {
        let source = parse(src);
        let reads = free_reads(&source.ast().body[0]);
        assert!(!reads.contains(bound), "{bound} binds inside the body");
        assert!(
            reads.contains("Outside"),
            "a name the body never binds stays a read"
        );
    }

    #[test]
    fn call_reachable_follows_call_edges_to_a_fixed_point() {
        let source = parse(concat!(
            "def leaf():\n    return LEAF_READ\n\n\n",
            "def middle():\n    return leaf()\n\n\n",
            "def top():\n    return middle()\n"
        ));
        let reach = call_reachable(&source.ast().body);
        assert!(
            reach["middle"].contains("LEAF_READ"),
            "one call edge reaches the leaf read"
        );
        assert!(
            reach["top"].contains("LEAF_READ"),
            "two call edges reach the leaf read"
        );
    }
}

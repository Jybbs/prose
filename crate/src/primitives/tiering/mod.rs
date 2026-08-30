//! Topological tiering of definition runs by evaluation-time
//! dependency, alongside the soundness check a reorder runs against
//! that same reference graph.

use std::ops::Range;

use itertools::Itertools;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::primitives::orderer::permute_in_place;

mod reach;
mod refs;
mod strands;
mod tiers;

use reach::called_names;
pub(crate) use reach::{CallReach, call_reachable, calls_a_name};
use refs::eval_time_refs;
pub(crate) use refs::{eval_refs, observed_refs, walk_lambda_defaults};
pub(crate) use strands::permute_or_repair;
pub(crate) use tiers::tier_levels;

/// The evaluation-time references and the evaluated names of a body,
/// the pair an [`Evaluation`] borrows.
pub(crate) struct Evaluated<'src> {
    names: FxHashMap<TextSize, Vec<&'src str>>,
    refs: FxHashMap<TextSize, Vec<&'src str>>,
}

impl<'src> Evaluated<'src> {
    /// The pair over `body`, widening through `reachable` and skipping
    /// annotations while `defer_annotations`.
    pub(crate) fn of(
        body: &'src [Stmt],
        reachable: &CallReach<'src>,
        defer_annotations: bool,
    ) -> Self {
        let refs = eval_time_refs_of(body, defer_annotations);
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
        self.names
            .get(&stmt.range().start())
            .map_or(&[], Vec::as_slice)
    }

    /// The evaluation-time references of `stmt`, empty for a statement
    /// outside the body the cache was built over.
    fn refs_of(self, stmt: &Stmt) -> &'a [&'src str] {
        self.refs
            .get(&stmt.range().start())
            .map_or(&[], Vec::as_slice)
    }
}

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
    evaluation: Evaluation<'_, 'src>,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
) -> Option<FxHashMap<TextSize, (usize, K)>> {
    let members: Vec<(&'src Stmt, &'src str, K)> = body
        .iter()
        .filter_map(|stmt| member(stmt).map(|(name, key)| (stmt, name, key)))
        .collect();
    let name_to_idx = unique_name_index(members.iter().map(|&(_, name, _)| name))?;
    let dep_sets: Vec<FxHashSet<usize>> = members
        .iter()
        .enumerate()
        .map(|(idx, &(stmt, _, _))| {
            evaluation
                .refs_of(stmt)
                .iter()
                .filter_map(|name| name_to_idx.get(name).copied())
                // A recursive self-reference does not constrain sibling order.
                .filter(|&dep| dep != idx)
                .collect()
        })
        .collect();
    let tiers = tier_levels(&dep_sets)?;
    Some(
        members
            .into_iter()
            .zip(tiers)
            .map(|((stmt, _, key), tier)| (stmt.range().start(), (tier, key)))
            .collect(),
    )
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
            (stmt.range().start(), refs)
        })
        .collect()
}

/// Tiers the `member`-selected definitions within `range` and permutes
/// those slots of `order` by `rank` over each member's tier and key,
/// leaving `order` untouched when the run declines. A member `holds`
/// selects keeps its source slot, and a member the permutation would
/// seat below a statement that names it holds its slot while the rest
/// of the run still sorts.
pub(crate) fn permute_defs<'src, K: Copy, R: Ord>(
    order: &mut [usize],
    body: &'src [Stmt],
    range: Range<usize>,
    evaluation: Evaluation<'_, 'src>,
    holds: impl Fn(&'src Stmt) -> bool,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
    rank: impl Fn(usize, K) -> R,
) {
    let Some(keys) = def_run_tier_keys(&body[range.clone()], evaluation, &member) else {
        return;
    };
    permute_or_repair(
        order,
        body,
        &range,
        evaluation,
        |stmt| member(stmt).map(|(name, _)| name),
        |order, pinned| {
            permute_in_place(order, body, range.clone(), |stmt| {
                keys.get(&stmt.range().start())
                    .copied()
                    .filter(|_| !holds(stmt) && !pinned.contains(&stmt.range().start()))
                    .map(|(tier, key)| rank(tier, key))
            })
        },
    );
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
            let own = refs
                .get(&stmt.range().start())
                .map_or(&[][..], Vec::as_slice);
            let called;
            let runs: &[&str] = if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
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
            (stmt.range().start(), names)
        })
        .collect()
}

/// Indexes each name to its position, or `None` when a name repeats. A
/// duplicate makes an intra-run reference ambiguous, so the caller
/// declines the reorder.
fn unique_name_index<'a>(
    names: impl Iterator<Item = &'a str>,
) -> Option<FxHashMap<&'a str, usize>> {
    let mut index = FxHashMap::default();
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

    use super::*;
    use crate::{
        primitives::decorator::is_decorated,
        testing::{evaluated, parse},
    };

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
        let evaluated = evaluated(&source, body);
        permute_defs(
            &mut order,
            body,
            0..body.len(),
            evaluated.evaluation(),
            holds,
            class_member,
            |tier, key| (tier, key),
        );
        order
    }

    /// The new-order permutation `permute_defs` produces over `src`'s
    /// function run, holding nothing.
    fn func_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let evaluated = evaluated(&source, body);
        permute_defs(
            &mut order,
            body,
            0..body.len(),
            evaluated.evaluation(),
            |_| false,
            |stmt| {
                stmt.as_function_def_stmt().map(|func| {
                    let name = func.name.as_str();
                    (name, name)
                })
            },
            |tier, key| (tier, key),
        );
        order
    }

    #[test]
    fn def_run_tier_keys_declines_a_duplicate_member_name() {
        let source = parse("class Dup:\n    pass\n\n\nclass Dup:\n    pass\n");
        assert!(
            def_run_tier_keys(
                &source.ast().body,
                evaluated(&source, &source.ast().body).evaluation(),
                class_member
            )
            .is_none()
        );
    }

    #[test]
    fn def_run_tier_keys_declines_a_reference_cycle() {
        let source = parse("class Alpha(Beta):\n    pass\n\n\nclass Beta(Alpha):\n    pass\n");
        assert!(
            def_run_tier_keys(
                &source.ast().body,
                evaluated(&source, &source.ast().body).evaluation(),
                class_member
            )
            .is_none()
        );
    }

    #[test]
    fn def_run_tier_keys_excludes_a_recursive_self_reference() {
        let source = parse("class Node:\n    def child(self) -> Node: ...\n");
        let body = &source.ast().body;
        let keys = def_run_tier_keys(
            body,
            evaluated(&source, &source.ast().body).evaluation(),
            class_member,
        )
        .expect("self-reference does not decline");
        assert_eq!(keys[&body[0].range().start()].0, 0);
    }

    #[test]
    fn def_run_tier_keys_tiers_a_backward_base_class_reference() {
        let source = parse("class Beta:\n    pass\n\n\nclass Alpha(Beta):\n    pass\n");
        let body = &source.ast().body;
        let keys = def_run_tier_keys(
            body,
            evaluated(&source, &source.ast().body).evaluation(),
            class_member,
        )
        .expect("acyclic run tiers");
        let tier = |i: usize| keys[&body[i].range().start()].0;
        assert_eq!(tier(0), 0, "Beta has no dependency");
        assert_eq!(tier(1), 1, "Alpha depends on Beta");
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
    fn permute_defs_holds_a_definition_its_decorator_reaches() {
        let src = indoc! {"
            def zzz_helper():
                return 1

            def deco(cls):
                zzz_helper()
                return cls

            @deco
            class Alpha:
                pass

            def aaa():
                pass
        "};
        assert_eq!(
            func_order(src),
            vec![0, 1, 2, 3],
            "decorating Alpha runs deco, which reads zzz_helper, so both hold above it"
        );
    }

    #[test]
    fn permute_defs_holds_a_function_a_method_call_on_a_class_reaches() {
        let src = indoc! {"
            def zzz_check(candidate):
                return candidate

            class Zed:
                def hook(self):
                    return zzz_check(self)

            Zed.register(int)

            def alpha():
                pass
        "};
        assert_eq!(
            func_order(src),
            vec![0, 1, 2, 3],
            "the attribute call reaches Zed's body, which reads zzz_check, so it holds above"
        );
    }

    #[test]
    fn permute_defs_pins_a_definition_a_module_level_call_reaches() {
        let src = indoc! {"
            def zeta():
                return 1

            def mid():
                return zeta()

            mid()

            def delta():
                pass

            def beta():
                pass
        "};
        assert_eq!(
            func_order(src),
            vec![0, 1, 2, 4, 3],
            "the call holds zeta and mid while delta and beta still sort"
        );
    }

    #[test]
    fn permute_defs_pins_through_a_two_hop_call_chain() {
        let src = indoc! {"
            def zeta():
                return 1

            def inner():
                return zeta()

            def outer():
                return inner()

            outer()

            def delta():
                pass

            def beta():
                pass
        "};
        assert_eq!(
            func_order(src),
            vec![0, 1, 2, 3, 5, 4],
            "the reach of outer covers zeta two hops down"
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
    fn permute_defs_sorts_past_a_call_reaching_nothing_in_the_run() {
        let src = indoc! {"
            def zeta():
                return 1

            unrelated()

            def alpha():
                pass
        "};
        assert_eq!(
            func_order(src),
            vec![2, 1, 0],
            "a call into no definition of the run constrains nothing"
        );
    }
}

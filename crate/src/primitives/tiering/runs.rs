//! One definition run prepared for permutation, tiering its members
//! through the shared dependency graph once so a caller permuting the
//! same range on every pass of a fixed-point loop pays for the tiering
//! and the binder graph a single time.

use std::ops::Range;

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Evaluation, Strands, tiers::tier_levels};
use crate::primitives::orderer::permute_in_place;

/// One definition run prepared for permutation, holding the tier keys
/// of its members beside the binder graph a repair reads. Both are
/// fixed for the run, so a caller permuting the same range on every
/// pass of a fixed-point loop builds them once rather than per pass.
pub(crate) struct DefRun<'a, 'src, K> {
    keys: FxHashMap<TextSize, (usize, K)>,
    range: Range<usize>,
    strands: Strands<'a, 'src>,
}

impl<'a, 'src, K: Copy> DefRun<'a, 'src, K> {
    /// Prepares the `member`-selected definitions within `range`, `None`
    /// where the run cannot reorder because a name repeats or the
    /// reference graph carries a cycle.
    pub(crate) fn of(
        body: &'src [Stmt],
        range: Range<usize>,
        evaluation: Evaluation<'a, 'src>,
        member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
    ) -> Option<Self> {
        let keys = def_run_tier_keys(&body[range.clone()], evaluation, &member)?;
        let strands = Strands::of(body, &range, evaluation, |stmt| {
            member(stmt).map(|(name, _)| name)
        });
        Some(Self {
            keys,
            range,
            strands,
        })
    }

    /// Permutes this run's slots of `order` by `rank` over each member's
    /// tier and key, leaving `order` untouched when the run declines. A
    /// member `holds` selects keeps its source slot, and a member the
    /// permutation would seat below a statement that names it holds its
    /// slot while the rest of the run still sorts.
    pub(crate) fn permute<R: Ord>(
        &self,
        order: &mut [usize],
        body: &'src [Stmt],
        holds: impl Fn(&'src Stmt) -> bool,
        rank: impl Fn(usize, K) -> R,
    ) {
        self.strands
            .permute_or_repair(order, self.range.len(), |order, pinned| {
                permute_in_place(order, body, self.range.clone(), |stmt| {
                    self.keys
                        .get(&stmt.range().start())
                        .copied()
                        .filter(|_| !holds(stmt) && !pinned.contains(&stmt.range().start()))
                        .map(|(tier, key)| rank(tier, key))
                })
            });
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
    // A repeated name makes an intra-run reference ambiguous, so the run
    // declines rather than guessing which member a reference meant.
    let name_to_idx: FxHashMap<&str, usize> = members
        .iter()
        .enumerate()
        .map(|(at, &(_, name, _))| (name, at))
        .collect();
    if name_to_idx.len() != members.len() {
        return None;
    }
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

    fn func_member(stmt: &Stmt) -> Option<(&str, &str)> {
        stmt.as_function_def_stmt().map(|func| {
            let name = func.name.as_str();
            (name, name)
        })
    }

    /// The new-order permutation a [`DefRun`] produces over `src`'s
    /// class run, holding each member `holds` selects.
    fn class_order(src: &str, holds: fn(&Stmt) -> bool) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let evaluated = evaluated(&source, body);
        if let Some(run) = DefRun::of(body, 0..body.len(), evaluated.evaluation(), class_member) {
            run.permute(&mut order, body, holds, |tier, key| (tier, key));
        }
        order
    }

    /// The new-order permutation a [`DefRun`] produces over `src`'s
    /// function run, holding nothing.
    fn func_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let evaluated = evaluated(&source, body);
        if let Some(run) = DefRun::of(body, 0..body.len(), evaluated.evaluation(), func_member) {
            run.permute(&mut order, body, |_| false, |tier, key| (tier, key));
        }
        order
    }

    #[test]
    fn def_run_exempts_a_held_member_naming_itself() {
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
    fn def_run_holds_a_decorated_definition() {
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
    fn def_run_holds_a_definition_a_constructor_reaches_through_self() {
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
    fn def_run_holds_a_definition_its_decorator_reaches() {
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
    fn def_run_holds_a_function_a_method_call_on_a_class_reaches() {
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
    fn def_run_pins_a_definition_a_module_level_call_reaches() {
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
    fn def_run_pins_through_a_two_hop_call_chain() {
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
    fn def_run_reverts_when_a_hold_strands_a_base_class() {
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
    fn def_run_sorts_past_a_call_reaching_nothing_in_the_run() {
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
}

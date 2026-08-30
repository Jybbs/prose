//! Kahn-style topological tiers over a run's intra-run dependency sets.

use std::collections::VecDeque;

use rustc_hash::FxHashSet;

use crate::primitives::group_map;

/// Assigns each binding a Kahn-style topological tier from its
/// intra-run dependency set. Tier 0 is bindings with no deps, tier N
/// is bindings whose deps all sit in tiers strictly less than N. `None`
/// for a cycle or a self-loop.
pub(crate) fn tier_levels(dep_sets: &[FxHashSet<usize>]) -> Option<Vec<usize>> {
    let dependents = group_map(
        dep_sets
            .iter()
            .enumerate()
            .flat_map(|(binding, deps)| deps.iter().map(move |&dep| (dep, binding))),
    );
    let mut pending: Vec<usize> = dep_sets.iter().map(FxHashSet::len).collect();
    let mut queue: VecDeque<usize> = (0..dep_sets.len()).filter(|&i| pending[i] == 0).collect();
    let mut tiers = vec![None; dep_sets.len()];
    while let Some(binding) = queue.pop_front() {
        tiers[binding] = Some(
            dep_sets[binding]
                .iter()
                .filter_map(|&dep| tiers[dep])
                .max()
                .map_or(0, |tier| tier + 1),
        );
        for &dependent in dependents.get(&binding).into_iter().flatten() {
            pending[dependent] -= 1;
            if pending[dependent] == 0 {
                queue.push_back(dependent);
            }
        }
    }
    tiers.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[test]
    fn tier_levels_assigns_zero_for_empty_deps() {
        let deps = vec![
            FxHashSet::default(),
            FxHashSet::default(),
            FxHashSet::default(),
        ];
        assert_eq!(tier_levels(&deps), Some(vec![0, 0, 0]));
    }

    #[test]
    fn tier_levels_climbs_through_chain() {
        let deps = vec![
            FxHashSet::default(),
            FxHashSet::from_iter([0]),
            FxHashSet::from_iter([1]),
            FxHashSet::from_iter([0, 2]),
        ];
        assert_eq!(tier_levels(&deps), Some(vec![0, 1, 2, 3]));
    }

    #[rstest]
    #[case(vec![FxHashSet::from_iter([0])])]
    #[case(vec![FxHashSet::from_iter([1]), FxHashSet::from_iter([0])])]
    #[case(vec![FxHashSet::from_iter([1]), FxHashSet::from_iter([2]), FxHashSet::from_iter([0])])]
    fn tier_levels_returns_none_on_cycles(#[case] deps: Vec<FxHashSet<usize>>) {
        assert_eq!(tier_levels(&deps), None);
    }

    proptest! {
        #[test]
        fn tier_levels_assigns_dependency_respecting_tiers_for_dags(
            deps in prop::collection::vec(prop::collection::vec(0usize..16, 0..4), 1..16),
        ) {
            let dag: Vec<FxHashSet<usize>> = deps
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
            let mut deps: Vec<FxHashSet<usize>> = (0..n).map(|_| FxHashSet::default()).collect();
            deps[cycle_index].insert(cycle_index);
            prop_assert_eq!(tier_levels(&deps), None);
        }
    }
}

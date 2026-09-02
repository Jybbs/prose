//! The soundness repair a permutation runs against its run's binders,
//! pinning each member the arrangement seats across a binding it
//! evaluates.

use std::{iter, ops::Range};

use itertools::Either;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use super::Evaluation;
use crate::primitives::{binding::module_bound_names, group_map, slots::slot_positions};

/// The binders and the evaluated names of one run, read against any
/// arrangement of it. Fixed for the run, so a caller permuting the same
/// range repeatedly builds it once.
pub(crate) struct Strands<'a, 'src> {
    bound_at: FxHashMap<&'src str, Vec<usize>>,
    pinnable: FxHashMap<usize, TextSize>,
    readers: Vec<(usize, &'a [&'src str])>,
}

impl<'a, 'src> Strands<'a, 'src> {
    /// The binders of the `member_name` members and of every other
    /// statement in `range`, the offset each member pins by, and the
    /// names each statement evaluates under `evaluation`.
    pub(crate) fn of(
        body: &'src [Stmt],
        range: &Range<usize>,
        evaluation: Evaluation<'a, 'src>,
        member_name: impl Fn(&'src Stmt) -> Option<&'src str>,
    ) -> Self {
        let slots = || body[range.clone()].iter().zip(range.clone());
        let bound_names = |stmt: &'src Stmt| match member_name(stmt) {
            Some(name) => Either::Left(iter::once(name)),
            None => Either::Right(module_bound_names(stmt).into_iter()),
        };
        Self {
            bound_at: group_map(
                slots().flat_map(|(stmt, at)| bound_names(stmt).map(move |name| (name, at))),
            ),
            pinnable: slots()
                .filter_map(|(stmt, at)| member_name(stmt).map(|_| (at, stmt.start())))
                .collect(),
            readers: slots()
                .map(|(stmt, at)| (at, evaluation.names(stmt)))
                .collect(),
        }
    }

    /// The start offsets of the members `order` seats across a binding
    /// they evaluate, being the members a repair pins, each crossed pair
    /// contributing whichever of its two sides is a member. An empty set
    /// means the arrangement strands nothing.
    fn stranded(&self, order: &[usize]) -> FxHashSet<TextSize> {
        let position = slot_positions(order);
        self.readers
            .iter()
            .flat_map(|(reader, names)| {
                names.iter().flat_map(move |name| {
                    self.bound_at
                        .get(name)
                        .into_iter()
                        .flatten()
                        .map(move |&binder| (binder, *reader))
                })
            })
            .filter(|&(binder, reader)| !side_kept(binder, reader, &position))
            .flat_map(|(binder, reader)| [self.pinnable.get(&binder), self.pinnable.get(&reader)])
            .flatten()
            .copied()
            .collect()
    }

    /// Runs `permute` against `order` and repairs the result until it
    /// strands nothing, re-running the permutation with every stranded
    /// member pinned to its pre-permutation slot. `permute` reads that
    /// pin set by definition start offset and declines each entry it
    /// holds. Restores the pre-permutation slots when the repair runs
    /// out of members to pin, `span` bounding how many it can pin.
    pub(crate) fn permute_or_repair(
        &self,
        order: &mut [usize],
        span: usize,
        mut permute: impl FnMut(&mut [usize], &FxHashSet<TextSize>) -> bool,
    ) {
        let snapshot = order.to_vec();
        let mut pinned: FxHashSet<TextSize> = FxHashSet::default();
        for _ in 0..=span {
            order.copy_from_slice(&snapshot);
            if !permute(order, &pinned) {
                break;
            }
            let stranded = self.stranded(order);
            if stranded.is_empty() {
                return;
            }
            pinned.extend(stranded);
        }
        order.copy_from_slice(&snapshot);
    }
}

/// True when `binding` stays on the side of `reader` that the source
/// seated it, so a reader neither rises above a binding it evaluates
/// nor crosses one the source placed after it. `position` inverts a
/// permutation, so a statement binding the name it reads compares equal
/// on both sides and imposes nothing on itself.
fn side_kept(binding: usize, reader: usize, position: &[usize]) -> bool {
    binding.cmp(&reader) == position[binding].cmp(&position[reader])
}

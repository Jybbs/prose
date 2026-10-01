//! The soundness repair a permutation runs against its run's binders,
//! pinning each member the arrangement seats across a binding it
//! evaluates, or below a constant it anchors.

use std::{iter, ops::Range};

use itertools::Either;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Evaluation, observed_refs};
use crate::primitives::{
    binding::{is_explicit_type_alias, module_bound_names, single_name_assignment},
    group_map,
    slots::slot_positions,
};

/// The binders and the evaluated names of one run, read against any
/// arrangement of it and fixed for the run, beside each pair of a member
/// and a constant below it that the member anchors.
pub(crate) struct Strands<'a, 'src> {
    anchors: Vec<(usize, usize)>,
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
            anchors: Vec::new(),
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
    /// they evaluate or below a constant they anchor, being the members a
    /// repair pins, each crossed pair contributing whichever of its two
    /// sides is a member. An empty set means the arrangement strands
    /// nothing.
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
            .chain(self.anchors.iter().copied())
            .filter(|&(above, below)| !side_kept(above, below, &position))
            .flat_map(|(above, below)| [self.pinnable.get(&above), self.pinnable.get(&below)])
            .flatten()
            .copied()
            .collect()
    }

    /// Holds each member above every later constant whose value observes
    /// a name the member reads at evaluation time through a subscript or
    /// an attribute, since `band-constants` anchors such a constant below
    /// that member.
    pub(super) fn anchor_observers(
        mut self,
        body: &'src [Stmt],
        evaluation: Evaluation<'a, 'src>,
    ) -> Self {
        for &(constant, _) in &self.readers {
            let Some((_, Some(value))) = single_name_assignment(&body[constant])
                .filter(|_| !is_explicit_type_alias(&body[constant]))
            else {
                continue;
            };
            let observed = observed_refs(value);
            self.anchors.extend(
                self.pinnable
                    .keys()
                    .filter(|&&member| {
                        member < constant
                            && evaluation
                                .refs_of(&body[member])
                                .iter()
                                .any(|name| observed.contains(name))
                    })
                    .map(|&member| (member, constant)),
            );
        }
        self
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
/// seated it. `position` inverts a permutation, so a statement binding
/// the name it reads compares equal on both sides and imposes nothing
/// on itself.
fn side_kept(binding: usize, reader: usize, position: &[usize]) -> bool {
    binding.cmp(&reader) == position[binding].cmp(&position[reader])
}

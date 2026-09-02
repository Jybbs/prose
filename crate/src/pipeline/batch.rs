//! The batch of rules whose fix groups splice into one buffer in a
//! single pass, and the fix groups each member contributes.

use itertools::Itertools;
use ruff_diagnostics::{Edit, SourceMap};
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_source_file::SourceFile;
use ruff_text_size::Ranged;

use super::{PipelineError, error::reparse_or_reject, filter::prepared_groups};
use crate::{primitives::edit::apply_edits_mapped, rule::Rule, source::Source};

/// The rules whose fix groups splice into one buffer in a single pass,
/// each read against the buffer the batch opened on.
#[derive(Default)]
pub(super) struct Batch<'a> {
    /// Every member's edits, sorted for the weave.
    edits: Vec<Edit>,
    /// Each member's seat beside its rule.
    members: Vec<(usize, &'a dyn Rule)>,
}

impl<'a> Batch<'a> {
    /// Weaves every member's edits into `source` and reparses once,
    /// `source` itself for an empty batch, carrying into the result the
    /// binding table where every member declares its edits leave it
    /// standing. A rejected splice of several members either replays
    /// them one at a time, so the failure names the rule whose own
    /// edits produce it, or surfaces as [`PipelineError::Batch`] naming
    /// them all.
    fn splice(
        self,
        mut source: Source,
        gate: Option<PythonVersion>,
        replays: bool,
    ) -> Result<Source, PipelineError> {
        let Some((&(_, first), rest)) = self.members.split_first() else {
            return Ok(source);
        };
        let (text, map) = weave_edits(&source, self.edits);
        let bindings = source.take_binding_analysis();
        let entry = source.entry_buffer();
        match reparse_or_reject(source, text, first.id(), &map, gate) {
            Ok(mut next) => {
                let declining = self
                    .members
                    .iter()
                    .find_map(|&(_, rule)| (!rule.preserves_bindings()).then_some(rule.id()));
                next.inherit(
                    bindings,
                    &map,
                    declining.unwrap_or(first.id()),
                    declining.is_none(),
                );
                Ok(next)
            }
            Err(_) if !rest.is_empty() && !replays => Err(PipelineError::Batch {
                rules: self.members.iter().map(|(_, rule)| rule.id()).collect(),
            }),
            Err(_) if !rest.is_empty() => {
                let replayed =
                    self.members
                        .iter()
                        .try_fold(rebuilt(entry), |source, &(seat, rule)| {
                            let Some(spliceable) = Spliceable::landing(rule, &source) else {
                                return Ok(source);
                            };
                            let mut alone = Self::default();
                            alone.push(seat, rule, spliceable.edits);
                            alone.splice(source, gate, replays)
                        });
                debug_assert!(
                    replayed.is_err(),
                    "invariant: a batch whose splice is rejected holds a rule whose own splice is rejected",
                );
                replayed
            }
            rejected => rejected,
        }
    }

    /// Splices the batch and empties it for the next one.
    pub(super) fn close(
        &mut self,
        source: Source,
        gate: Option<PythonVersion>,
        replays: bool,
    ) -> Result<Source, PipelineError> {
        std::mem::take(self).splice(source, gate, replays)
    }

    /// True where an edit in the sorted `edits` overlaps one held, the
    /// overlap the weave declines.
    pub(super) fn conflicts_with(&self, edits: &[Edit]) -> bool {
        overlapping(self.edits.iter().merge(edits))
    }

    /// Adds a member, merging its sorted `edits` into the held ones.
    pub(super) fn push(&mut self, seat: usize, rule: &'a dyn Rule, edits: Vec<Edit>) {
        self.members.push((seat, rule));
        self.edits = std::mem::take(&mut self.edits)
            .into_iter()
            .merge(edits)
            .collect();
    }

    /// True where every member sits at one of `seats`, so a rule
    /// sharing a splice with each of them joins.
    pub(super) fn shares_with(&self, seats: &[usize]) -> bool {
        self.members.iter().all(|(seat, _)| seats.contains(seat))
    }
}

/// A rule's surviving fix groups over one buffer, beside their edits
/// sorted for the weave.
pub(super) struct Spliceable {
    pub(super) edits: Vec<Edit>,
    pub(super) groups: Vec<Vec<Edit>>,
}

impl Spliceable {
    /// [`of`](Self::of) narrowed to a rule whose edits weave, the
    /// reading a fold takes because it leaves an overlapping rule
    /// unapplied.
    pub(super) fn landing(rule: &dyn Rule, source: &Source) -> Option<Self> {
        Self::of(rule, source).filter(Self::lands)
    }

    /// `rule`'s surviving fix groups over `source`, `None` where none
    /// survives. A byte-identical duplicate among their edits is the
    /// signature of a walk reaching one node twice.
    pub(super) fn of(rule: &dyn Rule, source: &Source) -> Option<Self> {
        let groups = prepared_groups(rule, source);
        if groups.is_empty() {
            return None;
        }
        let mut edits = groups.concat();
        edits.sort_unstable();
        debug_assert!(
            edits.iter().all_unique(),
            "rule `{}` emitted a duplicate edit, the signature of a walk reaching one node twice",
            rule.id(),
        );
        Some(Self { edits, groups })
    }

    /// True where no two of the edits overlap, the overlap the weave
    /// declines.
    pub(super) fn lands(&self) -> bool {
        !overlapping(&self.edits)
    }

    /// True where any edit changes the text it covers in `source`.
    pub(super) fn rewrites(&self, source: &Source) -> bool {
        self.edits
            .iter()
            .any(|edit| edit.content().unwrap_or_default() != source.slice(edit))
    }

    /// The text the edits weave into `source`.
    pub(super) fn woven(self, source: &Source) -> String {
        weave_edits(source, self.edits).0
    }
}

/// The source `entry`'s buffer parses to, the entry a rejected splice
/// replays its members against one at a time. That buffer parsed when
/// the batch opened on it, so this parse holds.
fn rebuilt((file, source_type): (SourceFile, PySourceType)) -> Source {
    Source::build_module(file.source_text().to_owned(), file.name(), source_type)
        .expect("invariant: the buffer a batch opened on parses")
}

/// True where an edit in `sorted` starts before the one ahead of it
/// ends, the overlap the weave declines. Two insertions at one offset
/// and an edit ending where the next begins both pass.
fn overlapping<'e>(sorted: impl IntoIterator<Item = &'e Edit>) -> bool {
    sorted
        .into_iter()
        .tuple_windows()
        .any(|(held, next)| next.start() < held.end())
}

/// Splices the sorted, non-overlapping `edits` into `source`, returning
/// the woven text beside the `SourceMap` of the weave.
fn weave_edits(source: &Source, edits: Vec<Edit>) -> (String, SourceMap) {
    apply_edits_mapped(source.text(), edits).expect("invariant: sorted edits with no overlap weave")
}

//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>` and a `Vec<TextRange>` of lint
//! ranges. The pipeline splices the edits of a batch of consecutive
//! rules into a fresh buffer in one pass, then reparses and confirms
//! the result still compiles before handing the new `Source` to the
//! next batch. Registration order follows the data dependency, seating
//! every rule that mutates a line's width, a group's member order, or
//! a statement's position ahead of every rule that reads one, and a
//! rule joins a batch only beside rules the registry declares it
//! independent of. The settle check re-applies the enabled rules to a
//! completed run's output and names every rule still editing it.

use std::{ops::Range, slice};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::PythonVersion;

use crate::{
    diagnostics::Diagnostic,
    rule::{Rule, RuleId, independent},
    source::Source,
};

mod batch;
mod error;
mod filter;
mod validity;

use batch::{Batch, Spliceable};
pub use error::PipelineError;
use filter::{prepared_groups, settled_lints};
use validity::compile_gate;

/// Ordered sequence of enabled rules, run against each source file.
pub struct Pipeline {
    rules: Vec<Box<dyn Rule>>,
    /// The earlier seats each seat's rule shares a splice with under
    /// [`Sharing::Declared`].
    shares: Vec<Vec<usize>>,
    sharing: Sharing,
    target_version: PythonVersion,
}

impl Pipeline {
    /// Constructs a pipeline that performs no rewrites.
    pub fn empty() -> Self {
        Self::from_rules(Vec::new())
    }

    pub(crate) fn from_rules(rules: Vec<Box<dyn Rule>>) -> Self {
        let shares = rules
            .iter()
            .enumerate()
            .map(|(seat, rule)| {
                let later = rule.id();
                rules[..seat]
                    .iter()
                    .positions(|earlier| independent(later.as_str(), earlier.id().as_str()))
                    .collect()
            })
            .collect();
        Self {
            rules,
            shares,
            sharing: Sharing::default(),
            target_version: PythonVersion::default(),
        }
    }

    /// Sets which rules a run lets share a splice and a parse.
    #[must_use]
    pub fn sharing(mut self, sharing: Sharing) -> Self {
        self.sharing = sharing;
        self
    }

    /// Sets the Python version the compile gate evaluates against.
    #[must_use]
    pub(crate) fn targeting(mut self, target_version: Option<PythonVersion>) -> Self {
        self.target_version = target_version.unwrap_or_default();
        self
    }

    /// The diagnostics [`diagnose`](Self::diagnose) collects, paired
    /// with the seat of the first rule holding a fix group against
    /// `source`, or `None` where no rule holds one. A fold over this
    /// same buffer reaches its first edit at that seat, so every rule
    /// ahead of it re-derives a result this pass already has, and a
    /// `None` leaves that fold nothing to apply.
    fn diagnosed(&self, source: &Source) -> (Vec<Diagnostic>, Option<usize>) {
        if source.suppression_map().file_is_suppressed() {
            return (Vec::new(), None);
        }
        let mut diagnostics = Vec::new();
        let mut edits_at = None;
        for (seat, rule) in self.rules.iter().enumerate() {
            let groups = prepared_groups(&**rule, source);
            if !groups.is_empty() {
                edits_at.get_or_insert(seat);
            }
            diagnostics.extend(format_diagnostics(&**rule, groups));
        }
        diagnostics.extend(settled_lints(&self.rules, source));
        (diagnostics, edits_at)
    }

    /// Folds each rule's edits into `source` in registration order
    /// across `seats`, splicing a batch of rules in one pass and
    /// reparsing between batches, and extending `diagnostics` with each
    /// rule's format findings when the caller supplies one. A batch
    /// closes ahead of a rule whose edits overlap one it holds, and
    /// ahead of a rule the run's [`Sharing`] keeps out of it.
    ///
    /// `seats` bounds the fold to the rules seated in that range.
    /// [`run_as_written`](Self::run_as_written) opens at the first seat
    /// a [`diagnosed`](Self::diagnosed) pass found editing, leaving
    /// every rule ahead of it with no fix group for this fold to
    /// re-derive, and [`format_span`](Self::format_span) resumes behind
    /// a prefix another fold produced. The compile gate reads the
    /// segment's entry source.
    ///
    /// # Errors
    ///
    /// Returns whichever `PipelineError` a rule's output draws from
    /// [`reparse_or_reject`](error::reparse_or_reject).
    fn fold_rules(
        &self,
        mut source: Source,
        mut diagnostics: Option<&mut Vec<Diagnostic>>,
        seats: Range<usize>,
    ) -> Result<Source, PipelineError> {
        let gate = compile_gate(&source, self.target_version);
        let replays = self.sharing == Sharing::Declared;
        let opens = seats.start;
        let mut batch = Batch::default();
        for (offset, rule) in self.rules[seats].iter().enumerate() {
            let seat = opens + offset;
            let joins = match self.sharing {
                Sharing::Always => true,
                Sharing::Declared => batch.shares_with(&self.shares[seat]),
                Sharing::Never => false,
            };
            if !joins {
                source = batch.close(source, gate, replays)?;
            }
            let Some(mut spliceable) = Spliceable::landing(&**rule, &source) else {
                continue;
            };
            if batch.conflicts_with(&spliceable.edits) {
                source = batch.close(source, gate, replays)?;
                let Some(reread) = Spliceable::landing(&**rule, &source) else {
                    continue;
                };
                spliceable = reread;
            }
            debug_assert!(
                spliceable.rewrites(&source),
                "rule `{}` emitted edits that produced identical text",
                rule.id(),
            );
            if let Some(collected) = diagnostics.as_deref_mut() {
                collected.extend(format_diagnostics(&**rule, spliceable.groups));
            }
            batch.push(seat, &**rule, spliceable.edits);
        }
        batch.close(source, gate, replays)
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.rules.len()
    }

    /// Collects every rule's diagnostics against `source` without
    /// applying edits or reparsing between rules, so each range stays
    /// valid against the original buffer. Format rules contribute one
    /// diagnostic per surviving fix group and lint rules their lint
    /// diagnostics, both filtered through the suppression map exactly as
    /// [`run`](Self::run) filters them.
    pub fn diagnose(&self, source: &Source) -> Vec<Diagnostic> {
        self.diagnosed(source).0
    }

    /// A rendering of every rule's settings, equal for two pipelines
    /// whose rules were constructed against selections they read
    /// alike.
    pub fn fingerprint(&self) -> String {
        format!("{:?}", self.rules)
    }

    /// One fingerprint per carried rule, in registration order, each
    /// equal to what the rule's own single-rule pipeline renders.
    pub fn fingerprints(&self) -> Vec<String> {
        self.rules
            .iter()
            .map(|rule| format!("{:?}", slice::from_ref(rule)))
            .collect()
    }

    /// Rewrites `source` through every enabled rule, skipping the
    /// diagnostics [`run`](Self::run) collects and the lint pass it
    /// closes on.
    ///
    /// # Errors
    ///
    /// Returns whichever `PipelineError` a rule's output draws from the
    /// reparse between rules.
    pub fn format(&self, source: Source) -> Result<Source, PipelineError> {
        if source.suppression_map().file_is_suppressed() {
            return Ok(source);
        }
        self.fold_rules(source, None, 0..self.rules.len())
    }

    /// Rewrites `source` through the rules seated in `seats`, resuming
    /// behind a prefix whose output the caller already holds. The
    /// compile gate reads the segment's entry source.
    ///
    /// # Errors
    ///
    /// Returns whichever `PipelineError` a rule's output draws from the
    /// reparse between rules.
    ///
    /// # Panics
    ///
    /// Panics when `seats` reaches past the rule count.
    pub fn format_span(
        &self,
        source: Source,
        seats: Range<usize>,
    ) -> Result<Source, PipelineError> {
        self.fold_rules(source, None, seats)
    }

    /// Returns every registered rule's id in a stable order.
    /// Surfaces the same registry that
    /// [`RuleId::from_str`](crate::rule::RuleId) consults.
    pub fn known_ids() -> &'static [RuleId] {
        crate::rule::KNOWN_IDS
    }

    /// This pipeline's enabled rule ids in registration order, the
    /// resolved selection that keys the check cache so two runs
    /// differing only in `--select` / `--ignore` key separately.
    pub(crate) fn rule_ids(&self) -> impl Iterator<Item = RuleId> + use<'_> {
        self.rules.iter().map(|rule| rule.id())
    }

    /// Runs each registered rule against `source` in order and
    /// returns the rewritten source paired with the diagnostics each
    /// rule emitted.
    ///
    /// Lint diagnostics are collected once the rewrites settle, so
    /// every lint range resolves against the returned source rather
    /// than against the buffer as it stood when its rule ran.
    ///
    /// File-level `# prose: off` short-circuits to identity. The
    /// suppression map otherwise drops each fix group holding a
    /// suppressed edit, drops an empty group, and filters lint
    /// diagnostics per-line (`# prose: ignore`). Alignment rules
    /// pre-exclude suppressed rows before grouping, so this
    /// group-level pass is a no-op for them.
    ///
    /// # Errors
    ///
    /// Returns `PipelineError::Reparse` when a rule's edit list produces
    /// text that does not re-parse as Python, `PipelineError::Compile`
    /// when it parses but no longer compiles, and `PipelineError::Cell`
    /// when a notebook cell that parsed on its own before the rule ran no
    /// longer does.
    pub fn run(&self, source: Source) -> Result<(Source, Vec<Diagnostic>), PipelineError> {
        if source.suppression_map().file_is_suppressed() {
            return Ok((source, Vec::new()));
        }
        let mut diagnostics = Vec::new();
        let source = self.fold_rules(source, Some(&mut diagnostics), 0..self.rules.len())?;
        diagnostics.extend(settled_lints(&self.rules, &source));
        Ok((source, diagnostics))
    }

    /// Rewrites `source` and returns it beside the diagnostics
    /// [`diagnose`](Self::diagnose) collects against the buffer as
    /// written, the pair a structured `format` reports.
    ///
    /// One walk over the rules serves both halves, in that the fold
    /// opens at the first rule the diagnose pass found editing and
    /// leaves every rule ahead of it to that pass, where a buffer no
    /// rule edits skips the fold outright. No lint pass runs against the
    /// rewritten buffer, because the reported diagnostics resolve
    /// against the source as written. Replaying the editing rules is
    /// also what surfaces one whose output fails to re-parse or to
    /// compile, so `check --validate` reads this in place of the full
    /// [`run`](Self::run) and keeps the rewrite for its settle check.
    ///
    /// # Errors
    ///
    /// Returns `PipelineError::Reparse` when a rule's edit list produces
    /// text that does not re-parse as Python, `PipelineError::Compile`
    /// when it parses but no longer compiles, and `PipelineError::Cell`
    /// when a notebook cell that parsed on its own before the rule ran no
    /// longer does.
    pub(crate) fn run_as_written(
        &self,
        source: Source,
    ) -> Result<(Source, Vec<Diagnostic>), PipelineError> {
        let (diagnostics, edits_at) = self.diagnosed(&source);
        let Some(first) = edits_at else {
            return Ok((source, diagnostics));
        };
        Ok((
            self.fold_rules(source, None, first..self.rules.len())?,
            diagnostics,
        ))
    }

    /// What one walk over `source` reads for the settle check, so the
    /// rules still editing and the rules reporting a fix the weave
    /// never lands come off the same fix groups. Reads whichever subset
    /// this pipeline carries, so a `--select` run answers for that
    /// subset alone, and a file-level `# prose: off` answers empty.
    pub fn settle_report(&self, source: &Source) -> SettleReport {
        let mut report = SettleReport::default();
        if source.suppression_map().file_is_suppressed() {
            return report;
        }
        for rule in &self.rules {
            let Some(spliceable) = Spliceable::of(&**rule, source) else {
                continue;
            };
            let rule_id = rule.id();
            if !spliceable.lands() || !spliceable.rewrites(source) {
                report.unlanded.push(rule_id);
                continue;
            }
            report.editing.push(rule_id);
            if report.witness.is_none() {
                report.witness = Some((rule_id, spliceable.woven(source)));
            }
        }
        report
    }

    /// One pipeline per rule this pipeline carries, in order, each
    /// holding its rule as this pipeline constructed it, so a rule that
    /// reads a sibling's flag keeps the answer this selection gave it.
    pub fn split(self) -> Vec<(RuleId, Self)> {
        let (sharing, target_version) = (self.sharing, self.target_version);
        self.rules
            .into_iter()
            .map(|rule| {
                let rule_id = rule.id();
                let alone = Self::from_rules(vec![rule])
                    .sharing(sharing)
                    .targeting(Some(target_version));
                (rule_id, alone)
            })
            .collect()
    }

    /// The enabled rules whose edits would still rewrite `source`,
    /// empty once the run has settled. A rule whose surviving groups do
    /// not splice, or splice back to the same text, is left out, and
    /// [`settle_report`](Self::settle_report) names those separately.
    pub fn unsettled(&self, source: &Source) -> Vec<RuleId> {
        self.settle_report(source).editing
    }
}

/// What a settle check reads off one walk over a completed run's
/// output.
#[derive(Debug, Default)]
pub struct SettleReport {
    /// The enabled rules whose edits still rewrite the buffer, in
    /// registration order.
    pub editing: Vec<RuleId>,
    /// The enabled rules holding a fix group that splices back to the
    /// same text or does not apply, in registration order.
    pub unlanded: Vec<RuleId>,
    /// The first editing rule and the text its edits weave, the
    /// rewrite a report shows.
    pub witness: Option<(RuleId, String)>,
}

/// Which rules a run lets share a splice and a parse with the editing
/// rules ahead of them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Sharing {
    /// Every rule, so a run reparses only where a rule's edits overlap
    /// one already batched, the reading the subset probe takes of a
    /// pair to test whether its edits are independent. A batch the
    /// reparse rejects surfaces as [`PipelineError::Batch`] rather
    /// than replaying.
    Always,
    /// The rules the registry's independence table declares, every
    /// other rule reading the tree the batch ahead of it left.
    #[default]
    Declared,
    /// No rule, so every editing rule reads the tree the rule ahead of
    /// it left, the fold the subset probe measures a batched pair
    /// against.
    Never,
}

/// The format diagnostics `rule`'s surviving fix groups emit, one per
/// group.
fn format_diagnostics(rule: &dyn Rule, groups: Vec<Vec<Edit>>) -> impl Iterator<Item = Diagnostic> {
    let (rule_id, message) = (rule.id(), rule.message());
    groups
        .into_iter()
        .map(move |group| Diagnostic::format(rule_id, group, message.to_owned()))
}

#[cfg(test)]
mod tests;

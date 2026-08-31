//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>` and a `Vec<TextRange>` of lint
//! ranges. The pipeline sorts and applies the edits into a fresh
//! buffer, then reparses and confirms the result still compiles before
//! handing the new `Source` to the next rule, carrying into it the
//! tables the rule declares its edits leave standing. Registration
//! order follows the data dependency, seating every rule that mutates
//! a line's width, a group's member order, or a statement's position
//! ahead of every rule that reads one. The settle check re-applies the
//! enabled rules to a completed run's output and names every rule
//! still editing it, a `format` run re-applying only the rules that
//! edited on its first pass.

use std::collections::BTreeSet;

use ruff_diagnostics::{Edit, SourceMap};
use ruff_python_ast::PythonVersion;
use ruff_text_size::Ranged;

use crate::{
    diagnostics::Diagnostic,
    primitives::edit::apply_edits_mapped,
    rule::{Rule, RuleId},
    source::Source,
};

mod error;
mod filter;
mod validity;

pub use error::PipelineError;
use error::reparse_or_reject;
use filter::{prepared_groups, settled_lints};
use validity::compile_gate;

/// Ordered sequence of enabled rules, run against each source file.
pub struct Pipeline {
    rules: Vec<Box<dyn Rule>>,
    target_version: PythonVersion,
}

impl Pipeline {
    /// Constructs a pipeline that performs no rewrites.
    pub fn empty() -> Self {
        Self::from_rules(Vec::new())
    }

    pub(crate) fn from_rules(rules: Vec<Box<dyn Rule>>) -> Self {
        Self {
            rules,
            target_version: PythonVersion::default(),
        }
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

    /// Folds each rule's edits into `source` in registration order from
    /// the `first` seat onward, reparsing between rules and extending
    /// `diagnostics` with each rule's format findings when the caller
    /// supplies one.
    ///
    /// `first` is the seat a [`diagnosed`](Self::diagnosed) pass over
    /// this same buffer found editing before any other, leaving every
    /// rule ahead of it with no fix group for this fold to re-derive. A
    /// caller with the whole roster to fold passes zero.
    ///
    /// # Errors
    ///
    /// Returns whichever `PipelineError` a rule's output draws from
    /// [`reparse_or_reject`].
    fn fold_rules(
        &self,
        source: Source,
        mut diagnostics: Option<&mut Vec<Diagnostic>>,
        first: usize,
    ) -> Result<Source, PipelineError> {
        let gate = compile_gate(&source, self.target_version);
        self.rules[first..].iter().try_fold(source, |source, rule| {
            let Some((groups, new_text, map)) = woven_groups(&**rule, &source) else {
                return Ok(source);
            };
            debug_assert!(
                new_text != source.text(),
                "rule `{}` emitted edits that produced identical text",
                rule.id(),
            );
            if let Some(collected) = diagnostics.as_deref_mut() {
                collected.extend(format_diagnostics(&**rule, groups));
            }
            reparse_or_reject(source, new_text, &**rule, &map, gate)
        })
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.rules.len()
    }

    /// The rules `keep` admits whose edits would still rewrite `source`,
    /// each application reported to the trace under `pass`, empty under
    /// a file-level `# prose: off`. A rule whose surviving groups do not
    /// splice, or splice back to the same text, is left out.
    fn still_editing(
        &self,
        source: &Source,
        pass: &'static str,
        keep: impl Fn(RuleId) -> bool,
    ) -> Vec<RuleId> {
        if source.suppression_map().file_is_suppressed() {
            return Vec::new();
        }
        self.rules
            .iter()
            .filter(|rule| keep(rule.id()))
            .filter_map(|rule| {
                crate::source::trace::reapplied(pass, rule.id());
                let (_, text, _) = woven_groups(&**rule, source)?;
                (text != source.text()).then(|| rule.id())
            })
            .collect()
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
        let source = self.fold_rules(source, Some(&mut diagnostics), 0)?;
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
        Ok((self.fold_rules(source, None, first)?, diagnostics))
    }

    /// The enabled rules whose edits would still rewrite `source`,
    /// empty once the run has settled. Reads whichever subset this
    /// pipeline carries, so a `--select` run answers for that subset
    /// alone, and a file-level `# prose: off` answers empty. A rule
    /// whose surviving groups do not splice, or splice back to the same
    /// text, is left out.
    pub fn unsettled(&self, source: &Source) -> Vec<RuleId> {
        self.still_editing(source, "full", |_| true)
    }

    /// The rules among `fired` whose edits would still rewrite `source`,
    /// the second pass a `format` run makes over its own output,
    /// re-applying the rules that edited on the first pass rather than
    /// every enabled rule. A rule silent on the first pass is left to
    /// the full [`unsettled`](Self::unsettled) walk that `check
    /// --validate` and the settle sweeps run.
    pub(crate) fn unsettled_among(&self, source: &Source, fired: &BTreeSet<RuleId>) -> Vec<RuleId> {
        self.still_editing(source, "narrowed", |id| fired.contains(&id))
    }
}

/// True when no two edits across `groups` match on both range and
/// content. A byte-identical duplicate is the signature of a walk
/// reaching one node twice, whereas two differing edits over one span
/// are the overlap the weave declines on its own.
fn distinct_edits(groups: &[Vec<Edit>]) -> bool {
    let mut edits: Vec<&Edit> = groups.iter().flatten().collect();
    edits.sort_by_key(|edit| (edit.start(), edit.end()));
    edits.windows(2).all(|pair| pair[0] != pair[1])
}

/// The format diagnostics `rule`'s surviving fix groups emit, one per
/// group.
fn format_diagnostics(rule: &dyn Rule, groups: Vec<Vec<Edit>>) -> impl Iterator<Item = Diagnostic> {
    let (rule_id, message) = (rule.id(), rule.message());
    groups
        .into_iter()
        .map(move |group| Diagnostic::format(rule_id, group, message.to_owned()))
}

/// Applies `rule` to `source` and weaves its surviving fix groups into
/// new text beside the `SourceMap` of the weave, returning `None` when
/// no group survives or the edits do not apply.
fn woven_groups(rule: &dyn Rule, source: &Source) -> Option<(Vec<Vec<Edit>>, String, SourceMap)> {
    let groups = prepared_groups(rule, source);
    if groups.is_empty() {
        return None;
    }
    debug_assert!(
        distinct_edits(&groups),
        "rule `{}` emitted a duplicate edit, the signature of a walk reaching one node twice",
        rule.id(),
    );
    let (new_text, map) = apply_edits_mapped(source.text(), groups.concat())?;
    Some((groups, new_text, map))
}

#[cfg(test)]
mod tests;

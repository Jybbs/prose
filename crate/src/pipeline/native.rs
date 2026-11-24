//! The pipeline surface only the native build reads, meaning the one
//! trip a settle report shows, the rule slugs that key the check cache,
//! and the as-written run `check --validate` makes.

use super::{Pipeline, PipelineError, Trips};
use crate::{
    diagnostics::{Diagnostic, fired_rules},
    rules::RuleId,
    source::Source,
};

impl Pipeline {
    /// Rewrites `source` through every enabled rule once, the further
    /// trip a settle report shows over a run's output.
    ///
    /// # Errors
    ///
    /// Returns the `PipelineError` the reparse between rules reports
    /// for a rule's output.
    pub(crate) fn format_once(&self, source: Source) -> Result<Source, PipelineError> {
        self.format_span(source, 0..self.rules.len())
    }

    /// This pipeline's enabled rule slugs in registration order, the
    /// resolved selection that keys the check cache.
    pub(crate) fn rule_ids(&self) -> impl Iterator<Item = RuleId> + use<'_> {
        self.rules.iter().map(|rule| rule.id())
    }

    /// Rewrites `source` and returns it beside the diagnostics
    /// [`diagnose`](Self::diagnose) collects against the buffer as
    /// written, the pair a structured `format` reports, and what the
    /// run's trips came to. The fired set a narrowed settle check
    /// re-applies includes a rule that edits only once an upstream rule
    /// has rewritten, which the as-written diagnostics leave out.
    ///
    /// The first trip opens at the first rule the diagnose pass found
    /// editing and leaves every rule ahead of it to that pass, and a
    /// buffer no rule edits skips the trips outright. No lint pass runs
    /// against the rewritten buffer, leaving the reported diagnostics
    /// resolved against the source as written. `check --validate` reads
    /// this in place of the full [`run`](Self::run) and keeps the rewrite
    /// for its settle check.
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
    ) -> Result<(Source, Vec<Diagnostic>, Trips), PipelineError> {
        let (diagnostics, edits_at) = self.diagnosed(&source);
        let Some(first) = edits_at else {
            return Ok((source, diagnostics, Trips::default()));
        };
        let mut fold = Vec::new();
        let (formatted, capped) = self.fold_trips(source, Some(&mut fold), first)?;
        let fired = fired_rules(&fold);
        Ok((formatted, diagnostics, Trips { capped, fired }))
    }
}

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
    /// the `first` seat onward, splicing a batch of rules in one pass
    /// and reparsing between batches, and extending `diagnostics` with
    /// each rule's format findings when the caller supplies one. A
    /// batch closes ahead of a rule whose edits overlap one it holds,
    /// and ahead of a rule the run's [`Sharing`] keeps out of it.
    ///
    /// `first` is the seat a [`diagnosed`](Self::diagnosed) pass over
    /// this same buffer found editing before any other, leaving every
    /// rule ahead of it with no fix group for this fold to re-derive. A
    /// caller with the whole roster to fold passes zero.
    ///
    /// # Errors
    ///
    /// Returns whichever `PipelineError` a rule's output draws from
    /// [`reparse_or_reject`](error::reparse_or_reject).
    fn fold_rules(
        &self,
        mut source: Source,
        mut diagnostics: Option<&mut Vec<Diagnostic>>,
        first: usize,
    ) -> Result<Source, PipelineError> {
        let gate = compile_gate(&source, self.target_version);
        let replays = self.sharing == Sharing::Declared;
        let mut batch = Batch::default();
        for (seat, rule) in self.rules.iter().enumerate().skip(first) {
            let joins = match self.sharing {
                Sharing::Always => true,
                Sharing::Declared => batch.shares_with(&self.shares[seat]),
                Sharing::Never => false,
            };
            if !joins {
                source = batch.close(source, gate, replays)?;
            }
            let Some(mut spliceable) = Spliceable::of(&**rule, &source) else {
                continue;
            };
            if batch.conflicts_with(&spliceable.edits) {
                source = batch.close(source, gate, replays)?;
                let Some(reread) = Spliceable::of(&**rule, &source) else {
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

    /// The enabled rules whose edits would still rewrite `source`,
    /// empty once the run has settled. Reads whichever subset this
    /// pipeline carries, so a `--select` run answers for that subset
    /// alone, and a file-level `# prose: off` answers empty. A rule
    /// whose surviving groups do not splice, or splice back to the same
    /// text, is left out.
    pub fn unsettled(&self, source: &Source) -> Vec<RuleId> {
        if source.suppression_map().file_is_suppressed() {
            return Vec::new();
        }
        self.rules
            .iter()
            .filter(|rule| {
                Spliceable::of(rule.as_ref(), source).is_some_and(|s| s.rewrites(source))
            })
            .map(|rule| rule.id())
            .collect()
    }
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
mod tests {
    use std::{
        assert_matches,
        sync::{Arc, Mutex},
    };

    use itertools::Itertools;
    use ruff_diagnostics::Edit;
    use ruff_text_size::{TextLen, TextRange, TextSize};

    use super::*;
    use crate::{
        config::Config,
        diagnostics::Severity,
        primitives::edit::singleton_groups,
        rules::{
            align_colons::AlignColons, align_equals::AlignEquals,
            alphabetize_siblings::AlphabetizeSiblings,
        },
        testing::{
            FUTURE_LEAD, GroupSentinelRule, PrefixRule, assert_send_sync, breaks_compile,
            breaks_parse, never_settles, notebook, parse, range, self_overlapping,
        },
    };

    /// Test-only rule that emits `edit` only while the buffer opens
    /// with `guard`.
    struct GuardedRule {
        edit: Edit,
        guard: &'static str,
        id: RuleId,
    }

    impl Rule for GuardedRule {
        fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
            if source.text().starts_with(self.guard) {
                vec![vec![self.edit.clone()]]
            } else {
                Vec::new()
            }
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn message(&self) -> &'static str {
            "guarded test rule"
        }
    }

    /// Test-only lint-only rule that returns the range list supplied
    /// at construction and never produces edits.
    struct LintSentinelRule {
        id: RuleId,
        ranges: Vec<TextRange>,
    }

    impl Rule for LintSentinelRule {
        fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
            Vec::new()
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn lint(&self, _source: &Source) -> Vec<Diagnostic> {
            let rule = self.id;
            let message = self.message();
            self.ranges
                .iter()
                .map(|&range| Diagnostic::lint(rule, range, message.to_owned()))
                .collect()
        }

        fn message(&self) -> &'static str {
            "lint test rule"
        }
    }

    /// Test-only lint-only rule that locates `needle` in the source it
    /// is handed and emits one lint over it, so its range tracks the
    /// buffer the rule actually reads rather than a fixed offset.
    struct NeedleLintRule {
        id: RuleId,
        needle: &'static str,
    }

    impl Rule for NeedleLintRule {
        fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
            Vec::new()
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn lint(&self, source: &Source) -> Vec<Diagnostic> {
            let start = source.text().find(self.needle).expect("needle is present") as u32;
            let found = range(start, start + self.needle.len() as u32);
            vec![Diagnostic::lint(self.id, found, self.message().to_owned())]
        }

        fn message(&self) -> &'static str {
            "needle lint test rule"
        }
    }

    /// Test-only rule that records its own id into a shared log and
    /// never produces edits.
    struct SentinelRule {
        id: RuleId,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Rule for SentinelRule {
        fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
            self.log.lock().expect("log mutex").push(self.id.as_str());
            Vec::new()
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn message(&self) -> &'static str {
            "test rule"
        }
    }

    /// Test-only rule that captures `source.text()` at apply time and
    /// returns the edit list supplied at construction.
    struct TextCapturingRule {
        edits: Vec<Edit>,
        id: RuleId,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl Rule for TextCapturingRule {
        fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
            self.seen
                .lock()
                .expect("seen mutex")
                .push(source.text().to_owned());
            singleton_groups(self.edits.clone())
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn message(&self) -> &'static str {
            "test rule"
        }
    }

    fn registered_slugs(pipeline: &Pipeline) -> Vec<&'static str> {
        pipeline.rule_ids().map(|id| id.as_str()).collect()
    }

    #[test]
    fn compile_failure_surfaces_rule_id() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_compile())]);
        let source = parse(FUTURE_LEAD);

        let err = pipeline.run(source).expect_err("compile check should fail");

        assert_matches!(err, PipelineError::Compile { rule, .. } if rule.as_str() == "breaks-compile");
    }

    #[test]
    fn diagnose_collects_against_the_original_buffer_without_rewriting() {
        // The first rule would rewrite `x` to `y`, the second lints the
        // original `x` at 0..1. `diagnose` must not apply the first
        // rule's edit, so the lint range stays valid against the
        // untouched buffer and both findings surface together.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
                id: RuleId::from("rewrite-x-to-y"),
            }),
            Box::new(LintSentinelRule {
                id: RuleId::from("flag-x"),
                ranges: vec![range(0, 1)],
            }),
        ]);
        let source = parse("x = 1\n");

        let diagnostics = pipeline.diagnose(&source);

        assert_eq!(diagnostics.len(), 2);
        let format = diagnostics
            .iter()
            .find(|d| d.severity.is_format())
            .expect("format finding");
        assert_eq!(format.rule.as_str(), "rewrite-x-to-y");
        let lint = diagnostics
            .iter()
            .find(|d| d.severity.is_lint())
            .expect("lint finding");
        assert_eq!(lint.rule.as_str(), "flag-x");
        assert_eq!(lint.range, range(0, 1));
    }

    #[test]
    fn diagnose_drops_a_lint_under_a_per_line_ignore_directive() {
        // A bare `# prose: ignore` suppresses every rule on its line, so
        // the lint at `x` (line 1) is dropped through diagnose's
        // lint-suppression tail rather than its file-level short-circuit.
        let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
            id: RuleId::from("flag-x"),
            ranges: vec![range(0, 1)],
        })]);
        let source = parse("x = 1  # prose: ignore\n");

        assert!(pipeline.diagnose(&source).is_empty());
    }

    #[test]
    fn diagnose_drops_a_whole_group_holding_one_suppressed_edit() {
        // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
        //         |0--------|11----|17--------|27----|33
        // The group bundles an edit at 11..16 (inside the suppressed
        // [0..17) span) with one at 27..32. The group drops as a unit,
        // so diagnose emits nothing.
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![
                Edit::range_replacement("y".to_owned(), range(11, 16)),
                Edit::range_replacement("Z".to_owned(), range(27, 32)),
            ]],
            id: RuleId::from("rewrite-x-and-z"),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

        assert!(pipeline.diagnose(&source).is_empty());
    }

    #[test]
    fn diagnose_drops_findings_under_a_suppressed_span() {
        let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
            id: RuleId::from("flag-stuff"),
            ranges: vec![range(13, 14)],
        })]);
        let source = parse("# prose: off\nx = 1\n");

        assert!(pipeline.diagnose(&source).is_empty());
    }

    #[test]
    fn diagnosed_names_no_seat_where_no_rule_edits() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(LintSentinelRule {
                id: RuleId::from("flag-x"),
                ranges: vec![range(0, 1)],
            }),
            Box::new(GroupSentinelRule {
                groups: vec![Vec::new()],
                id: RuleId::from("emits-empty-group"),
            }),
        ]);

        let (diagnostics, edits_at) = pipeline.diagnosed(&parse("x = 1\n"));

        assert_eq!(edits_at, None);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn diagnosed_stops_at_the_first_rule_holding_a_fix_group() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![Vec::new()],
                id: RuleId::from("emits-empty-group"),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
                id: RuleId::from("rewrite-x-to-y"),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("2".to_owned(), range(4, 5))]],
                id: RuleId::from("rewrite-1-to-2"),
            }),
        ]);

        let (_, edits_at) = pipeline.diagnosed(&parse("x = 1\n"));

        assert_eq!(edits_at, Some(1));
    }

    #[test]
    fn empty_pipeline_returns_identical_source() {
        let pipeline = Pipeline::from_rules(Vec::new());
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline.run(source).expect("identity run succeeds");

        assert_eq!(result.text(), "x = 1\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn from_rules_seats_each_rule_beside_the_earlier_seats_it_shares_a_splice_with() {
        let seated = |slug: &'static str| -> Box<dyn Rule> {
            Box::new(GroupSentinelRule {
                groups: Vec::new(),
                id: RuleId::from(slug),
            })
        };
        let pipeline = Pipeline::from_rules(vec![
            seated("strip-trailing-commas"),
            seated("bare-imports"),
            seated("align-equals"),
        ]);

        let seats = |later: &str, earlier: &[&str]| -> Vec<usize> {
            earlier
                .iter()
                .positions(|slug| independent(later, slug))
                .collect()
        };

        assert_eq!(
            pipeline.shares,
            [
                vec![],
                seats("bare-imports", &["strip-trailing-commas"]),
                seats("align-equals", &["strip-trailing-commas", "bare-imports"]),
            ],
        );
    }

    #[test]
    fn known_ids_matches_with_defaults_registration() {
        let config = Config::default();
        let pipeline = Pipeline::with_defaults(&config);
        let mut registered = registered_slugs(&pipeline);
        registered.sort_unstable();
        let mut known: Vec<&'static str> =
            Pipeline::known_ids().iter().map(RuleId::as_str).collect();
        known.sort_unstable();
        assert_eq!(registered, known);
    }

    #[test]
    fn pipeline_is_send_and_sync() {
        assert_send_sync::<Pipeline>();
    }

    #[test]
    fn reparse_failure_surfaces_rule_id() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        assert_matches!(
            pipeline.run(source),
            Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "breaks-parse"
        );
    }

    #[test]
    fn rules_run_in_registration_order() {
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(SentinelRule {
                id: RuleId::from("first"),
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                id: RuleId::from("second"),
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                id: RuleId::from("third"),
                log: log.clone(),
            }),
        ]);
        let source = parse("x = 1\n");

        pipeline.run(source).expect("all rules succeed");

        assert_eq!(
            *log.lock().expect("log mutex"),
            ["first", "second", "third"]
        );
    }

    #[test]
    fn run_applies_a_reordering_rule_on_a_notebook() {
        // A sibling reorder runs cell-aware on a notebook, so its
        // rewrite lands inside the cell that holds the members.
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);
        let source = notebook(&["x = 1"]);

        let (result, diagnostics) = pipeline.run(source).expect("notebook run succeeds");

        assert_eq!(result.text(), "y = 1\n");
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn run_as_written_leaves_the_unedited_prefix_to_the_diagnose_pass() {
        // The first rule reads the buffer and edits nothing, so the
        // diagnose pass has already answered for it and the fold opens
        // at the second. Its capture log holds the one read.
        let seen = Arc::new(Mutex::new(Vec::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: Vec::new(),
                id: RuleId::from("reads-only"),
                seen: Arc::clone(&seen),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
                id: RuleId::from("rewrite-x-to-y"),
            }),
        ]);

        let (formatted, _) = pipeline
            .run_as_written(parse("x = 1\n"))
            .expect("the run succeeds");

        assert_eq!(formatted.text(), "y = 1\n");
        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n"]);
    }

    #[test]
    fn run_as_written_passes_a_clean_rewrite() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);
        let source = parse("x = 1\n");

        assert!(pipeline.run_as_written(source).is_ok());
    }

    #[test]
    fn run_as_written_passes_an_overlapping_group_as_a_no_op() {
        let pipeline = Pipeline::from_rules(vec![Box::new(self_overlapping())]);
        let source = parse("x = 1\n");

        assert!(pipeline.run_as_written(source).is_ok());
    }

    #[test]
    fn run_as_written_passes_when_no_rule_edits() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![Vec::new()],
            id: RuleId::from("emits-empty-group"),
        })]);
        let source = parse("x = 1\n");

        assert!(pipeline.run_as_written(source).is_ok());
    }

    #[test]
    fn run_as_written_resolves_a_lint_range_against_the_original_buffer() {
        // `widen-x` moves the `1` one byte right, so the lint range the
        // rewritten buffer carries is 5..6 and the as-written one 4..5.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("yy".to_owned(), range(0, 1))]],
                id: RuleId::from("widen-x"),
            }),
            Box::new(NeedleLintRule {
                id: RuleId::from("flag-one"),
                needle: "1",
            }),
        ]);

        let (formatted, diagnostics) = pipeline
            .run_as_written(parse("x = 1\n"))
            .expect("the run succeeds");

        assert_eq!(formatted.text(), "yy = 1\n");
        let lint = diagnostics
            .iter()
            .find(|d| d.severity.is_lint())
            .expect("lint finding");
        assert_eq!(lint.range, range(4, 5));
    }

    #[test]
    fn run_as_written_returns_the_diagnostics_it_replayed() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);

        let (_, diagnostics) = pipeline
            .run_as_written(parse("x = 1\n"))
            .expect("the run succeeds");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-to-y");
    }

    #[test]
    fn run_as_written_short_circuits_when_file_is_suppressed() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(11, 12))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);
        let source = parse("# prose: off\nx = 1\n");

        let (formatted, diagnostics) = pipeline.run_as_written(source).expect("the run succeeds");

        assert_eq!(formatted.text(), "# prose: off\nx = 1\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn run_as_written_skips_the_replay_where_no_rule_edits() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let pipeline = Pipeline::from_rules(vec![Box::new(TextCapturingRule {
            edits: Vec::new(),
            id: RuleId::from("reads-only"),
            seen: Arc::clone(&seen),
        })]);

        pipeline
            .run_as_written(parse("x = 1\n"))
            .expect("the run succeeds");

        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n"]);
    }

    #[test]
    fn run_as_written_surfaces_uncompilable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_compile())]);
        let source = parse(FUTURE_LEAD);

        assert_matches!(
            pipeline.run_as_written(source),
            Err(PipelineError::Compile { rule, .. }) if rule.as_str() == "breaks-compile"
        );
    }

    #[test]
    fn run_as_written_surfaces_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        assert_matches!(
            pipeline.run_as_written(source),
            Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "breaks-parse"
        );
    }

    #[test]
    fn run_batches_a_declared_pair_against_one_buffer() {
        // `strip-trailing-commas` shares a splice with
        // `normalize-literals` in the registry's table, so the second
        // sentinel reads the buffer the first read.
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                id: RuleId::from("normalize-literals"),
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("2".to_owned(), range(4, 5))],
                id: RuleId::from("strip-trailing-commas"),
                seen: seen.clone(),
            }),
        ]);

        let (result, _) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

        assert_eq!(result.text(), "y = 2\n");
        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "x = 1\n"]);
    }

    #[test]
    fn run_batches_adjacent_edits_from_two_rules() {
        // An edit ending where the next begins is no overlap, so both
        // rules read the base buffer and land in one splice.
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("a".to_owned(), range(0, 1))],
                id: RuleId::from("rewrite-head"),
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("b".to_owned(), range(1, 2))],
                id: RuleId::from("rewrite-gap"),
                seen: seen.clone(),
            }),
        ])
        .sharing(Sharing::Always);

        let (result, _) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

        assert_eq!(result.text(), "ab= 1\n");
        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "x = 1\n"]);
    }

    #[test]
    fn run_batches_declared_pairs_alone() {
        // Neither sentinel is in the registry's independence table, so
        // the second reads the first's rewrite under the default
        // sharing, and the batch closes between them.
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                id: RuleId::from("rewrite-x-to-y"),
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                edits: Vec::new(),
                id: RuleId::from("downstream-observer"),
                seen: seen.clone(),
            }),
        ]);

        pipeline.run(parse("x = 1\n")).expect("both stages succeed");

        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "y = 1\n"]);
    }

    #[test]
    fn run_batches_independent_rules_against_one_buffer() {
        // The second reads the buffer the first read rather than the
        // first's rewrite, and both rewrites land in one splice.
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                id: RuleId::from("rewrite-x-to-y"),
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("2".to_owned(), range(4, 5))],
                id: RuleId::from("rewrite-1-to-2"),
                seen: seen.clone(),
            }),
        ])
        .sharing(Sharing::Always);

        let (result, diagnostics) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

        assert_eq!(result.text(), "y = 2\n");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "x = 1\n"]);
    }

    #[test]
    fn run_closes_a_batch_ahead_of_an_overlapping_edit() {
        // The second rule's edit covers the first's, so the batch
        // holding the first closes and the second re-reads the spliced
        // buffer before its own edit lands.
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                id: RuleId::from("rewrite-x-to-y"),
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("z".to_owned(), range(0, 1))],
                id: RuleId::from("rewrite-head-to-z"),
                seen: seen.clone(),
            }),
        ])
        .sharing(Sharing::Always);

        let (result, diagnostics) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

        assert_eq!(result.text(), "z = 1\n");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            *seen.lock().expect("seen mutex"),
            ["x = 1\n", "x = 1\n", "y = 1\n"],
        );
    }

    #[test]
    fn run_declines_an_overlapping_group_as_a_no_op() {
        let pipeline = Pipeline::from_rules(vec![Box::new(self_overlapping())]);
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline
            .run(source)
            .expect("overlap degrades, run continues");

        assert_eq!(result.text(), "x = 1\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn run_drops_a_rule_whose_edits_vanish_once_the_batch_closes() {
        // The second rule's edit overlaps the first's, so the batch
        // closes, and the spliced buffer no longer opens with `x`, so
        // its re-read emits nothing.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(PrefixRule {
                id: RuleId::from("rewrite-x-to-y"),
                reads: "x",
                writes: "y",
            }),
            Box::new(PrefixRule {
                id: RuleId::from("rewrite-x-to-z"),
                reads: "x",
                writes: "z",
            }),
        ])
        .sharing(Sharing::Always);

        let (result, diagnostics) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

        assert_eq!(result.text(), "y = 1\n");
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn run_drops_a_whole_group_holding_one_suppressed_edit() {
        // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
        //         |0--------|11----|17--------|27----|33
        // The group bundles an edit at 11..16 (inside the suppressed
        // [0..17) span) with one at 27..32. The group drops as a unit,
        // so the unsuppressed edit never applies alone.
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![
                Edit::range_replacement("y".to_owned(), range(11, 16)),
                Edit::range_replacement("Z".to_owned(), range(27, 32)),
            ]],
            id: RuleId::from("rewrite-x-and-z"),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

        let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nz = 9\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn run_drops_edits_whose_range_overlaps_a_suppressed_span() {
        // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
        //         |0--------|11----|17--------|27----|33
        // Edit at 11..16 (`x = 1`) sits inside the suppressed
        // [0..17) span and must be dropped, leaving the unsuppressed
        // edit at 27..32 (`z = 9`) in its own group to apply.
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: singleton_groups(vec![
                Edit::range_replacement("y".to_owned(), range(11, 16)),
                Edit::range_replacement("Z".to_owned(), range(27, 32)),
            ]),
            id: RuleId::from("rewrite-x-and-z"),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

        let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nZ\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-and-z");
    }

    #[test]
    fn run_emits_lint_diagnostic_without_fix_per_lint_range() {
        let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
            id: RuleId::from("flag-stuff"),
            ranges: vec![range(0, 5), range(6, 11)],
        })]);
        let source = parse("x = 1\ny = 2\n");

        let (result, diagnostics) = pipeline.run(source).expect("lint-only run succeeds");

        assert_eq!(result.text(), "x = 1\ny = 2\n");
        assert_eq!(diagnostics.len(), 2);
        for diagnostic in &diagnostics {
            assert_eq!(diagnostic.severity, Severity::Lint);
            assert!(diagnostic.fix.is_none());
            assert_eq!(diagnostic.rule.as_str(), "flag-stuff");
            assert_eq!(diagnostic.message, "lint test rule");
        }
    }

    #[test]
    fn run_emits_one_diagnostic_per_group_carrying_every_edit() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![
                Edit::range_replacement("Y".to_owned(), range(0, 1)),
                Edit::range_replacement("Z".to_owned(), range(4, 5)),
            ]],
            id: RuleId::from("rewrite-x-and-1"),
        })]);
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline.run(source).expect("grouped rewrite succeeds");

        assert_eq!(result.text(), "Y = Z\n");
        assert_eq!(diagnostics.len(), 1);
        let fix = diagnostics[0]
            .fix
            .as_ref()
            .expect("format diagnostic carries a fix");
        assert_eq!(fix.edits().len(), 2);
        assert_eq!(diagnostics[0].range, range(0, 5));
    }

    #[test]
    fn run_emits_one_diagnostic_per_surviving_edit() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline.run(source).expect("rewrite succeeds");

        assert_eq!(result.text(), "y = 1\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-to-y");
        assert_eq!(diagnostics[0].severity, Severity::Format);
        assert!(diagnostics[0].fix.is_some());
    }

    #[test]
    #[should_panic(expected = "emitted a duplicate edit")]
    fn run_flags_a_byte_identical_duplicate_edit() {
        let edit = Edit::range_replacement("y".to_owned(), range(0, 1));
        let rule = GroupSentinelRule {
            groups: vec![vec![edit.clone()], vec![edit]],
            id: RuleId::from("duplicating"),
        };
        let pipeline = Pipeline::from_rules(vec![Box::new(rule)]);
        let _ = pipeline.run(parse("x = 1\n"));
    }

    #[test]
    #[should_panic(expected = "invariant: a batch whose splice is rejected")]
    fn run_flags_a_replay_that_passes_where_its_batch_was_rejected() {
        // Spliced together the two rewrites demote the `__future__`
        // import and fail the gate, whereas replayed one at a time the
        // second rule sees `x = 1` and emits nothing, so the declared
        // pair has read each other's rewrite on this buffer.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement(
                    "x = 1".to_owned(),
                    range(0, 34),
                )]],
                id: RuleId::from("normalize-literals"),
            }),
            Box::new(GuardedRule {
                edit: Edit::range_replacement(
                    "from __future__ import division".to_owned(),
                    range(35, 44),
                ),
                guard: "from __future__",
                id: RuleId::from("strip-trailing-commas"),
            }),
        ]);
        let _ = pipeline.run(parse(FUTURE_LEAD));
    }

    #[test]
    fn run_forwards_a_notebook_through_one_batched_splice() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("xx".to_owned(), range(0, 1))]],
                id: RuleId::from("widen-x"),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("yy".to_owned(), range(7, 8))]],
                id: RuleId::from("widen-y"),
            }),
        ])
        .sharing(Sharing::Always);
        let source = notebook(&["x = 1\n", "y = 2\n"]);

        let (result, _) = pipeline.run(source).expect("notebook run succeeds");

        assert_eq!(result.text(), "xx = 1\n\nyy = 2\n\n");
        assert_eq!(result.cell_texts(), ["xx = 1\n", "yy = 2\n"]);
    }

    #[test]
    fn run_names_every_rule_of_a_batch_the_gate_rejects_under_always() {
        // The appended assignment and the demoting rewrite splice into
        // one buffer that parses and fails to compile, which the
        // probe's sharing reports as the batch rather than replaying.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::insertion(
                    "x = 1\n".to_owned(),
                    FUTURE_LEAD.text_len(),
                )]],
                id: RuleId::from("append-x"),
            }),
            Box::new(breaks_compile()),
        ])
        .sharing(Sharing::Always);

        assert_matches!(
            pipeline.run(parse(FUTURE_LEAD)),
            Err(PipelineError::Batch { rules })
                if rules == [RuleId::from("append-x"), RuleId::from("breaks-compile")]
        );
    }

    #[test]
    fn run_names_every_rule_of_a_batch_the_reparse_rejects_under_always() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(breaks_parse()),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("z".to_owned(), range(6, 7))]],
                id: RuleId::from("rewrite-y-to-z"),
            }),
        ])
        .sharing(Sharing::Always);

        assert_matches!(
            pipeline.run(parse("x = 1\ny = 2\n")),
            Err(PipelineError::Batch { rules })
                if rules == [RuleId::from("breaks-parse"), RuleId::from("rewrite-y-to-z")]
        );
    }

    #[test]
    fn run_names_the_rule_whose_output_a_declared_batch_replay_fails_to_compile() {
        // The demoting rewrite carries a slug sharing a splice with the
        // appending one, so the batch splices into one buffer that
        // fails the gate and the replay names the demoting rule alone.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::insertion(
                    "x = 1\n".to_owned(),
                    FUTURE_LEAD.text_len(),
                )]],
                id: RuleId::from("normalize-literals"),
            }),
            Box::new(GroupSentinelRule {
                groups: breaks_compile().groups,
                id: RuleId::from("strip-trailing-commas"),
            }),
        ]);

        assert_matches!(
            pipeline.run(parse(FUTURE_LEAD)),
            Err(PipelineError::Compile { rule, .. }) if rule.as_str() == "strip-trailing-commas"
        );
    }

    #[test]
    fn run_names_the_rule_whose_splice_a_declared_batch_replay_rejects() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: breaks_parse().groups,
                id: RuleId::from("normalize-literals"),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("z".to_owned(), range(6, 7))]],
                id: RuleId::from("strip-trailing-commas"),
            }),
        ]);

        assert_matches!(
            pipeline.run(parse("x = 1\ny = 2\n")),
            Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "normalize-literals"
        );
    }

    #[test]
    fn run_resolves_a_lint_range_against_the_settled_source() {
        // The lint rule registers ahead of the rewriting rule, which
        // inserts a line above the ignored statement. Collecting lints
        // after the rewrites settle keeps the lint's range on the row
        // carrying the directive, so the ignore still matches.
        let pipeline = Pipeline::from_rules(vec![
            Box::new(NeedleLintRule {
                id: RuleId::from("single-use-variables"),
                needle: "y = 2",
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::insertion(
                    "a = 0\n".to_owned(),
                    TextSize::new(0),
                )]],
                id: RuleId::from("prepend-a"),
            }),
        ]);
        let source = parse("x = 1\ny = 2  # prose: ignore[single-use-variables]\n");

        let (result, diagnostics) = pipeline.run(source).expect("prepend run succeeds");

        assert_eq!(
            result.text(),
            "a = 0\nx = 1\ny = 2  # prose: ignore[single-use-variables]\n",
        );
        assert!(diagnostics.iter().all(|d| !d.severity.is_lint()));
    }

    #[test]
    fn run_short_circuits_when_file_is_suppressed() {
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            id: RuleId::from("never-called"),
            log: log.clone(),
        })]);
        let source = parse("# prose: off\nx = 1\n");

        let (result, diagnostics) = pipeline.run(source).expect("short-circuit run");

        assert_eq!(result.text(), "# prose: off\nx = 1\n");
        assert!(diagnostics.is_empty());
        assert!(log.lock().expect("log mutex").is_empty());
    }

    #[test]
    fn run_skips_empty_group_without_emitting_a_diagnostic() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![Vec::new()],
            id: RuleId::from("emits-empty-group"),
        })]);
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline.run(source).expect("empty-group run succeeds");

        assert_eq!(result.text(), "x = 1\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn run_skips_reparse_when_every_edit_is_suppressed() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(11, 16))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\n");

        let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn run_skips_the_compile_gate_when_the_input_does_not_compile() {
        // The demoted `__future__` import arrives in the source, so the
        // rewrite of `os` to `sys` leaves the module exactly as
        // uncompilable as it was found and the run carries it through.
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("sys".to_owned(), range(7, 9))]],
            id: RuleId::from("rewrite-os-to-sys"),
        })]);
        let source = parse("import os\nfrom __future__ import annotations\n");

        let (result, _) = pipeline
            .run(source)
            .expect("disarmed gate lets the run pass");

        assert_eq!(
            result.text(),
            "import sys\nfrom __future__ import annotations\n"
        );
    }

    #[test]
    fn sharing_never_hands_a_downstream_rule_the_upstream_rewrite() {
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                id: RuleId::from("rewrite-x-to-y"),
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                edits: Vec::new(),
                id: RuleId::from("downstream-observer"),
                seen: seen.clone(),
            }),
        ])
        .sharing(Sharing::Never);
        let source = parse("x = 1\n");

        pipeline.run(source).expect("both stages succeed");

        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "y = 1\n"]);
    }

    #[test]
    fn unsettled_answers_empty_under_a_file_level_suppression() {
        let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
        let source = parse("# prose: off\nx = 1\n");

        assert!(pipeline.unsettled(&source).is_empty());
    }

    #[test]
    fn unsettled_names_a_rule_still_editing_a_notebook() {
        let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
        let source = notebook(&["x = 1\n", "y = 2\n"]);

        assert_eq!(pipeline.unsettled(&source), vec![RuleId::from("widener")]);
    }

    #[test]
    fn unsettled_names_only_the_rules_whose_edits_would_rewrite() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(never_settles("widener")),
            Box::new(GroupSentinelRule {
                groups: Vec::new(),
                id: RuleId::from("emits-nothing"),
            }),
            Box::new(self_overlapping()),
        ]);
        let source = parse("x = 1\n");

        assert_eq!(
            pipeline.unsettled(&source),
            vec![RuleId::from("widener")],
            "an empty group and an unspliceable one both leave the source settled",
        );
    }

    #[test]
    fn unsettled_reads_the_subset_the_pipeline_carries() {
        let source = parse("x = 1\n");
        let carried = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
        let bare = Pipeline::empty();

        assert_eq!(carried.unsettled(&source), vec![RuleId::from("widener")]);
        assert!(bare.unsettled(&source).is_empty());
    }

    #[test]
    fn unsettled_skips_a_rule_whose_edits_fall_in_a_suppressed_block() {
        let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
        let source = parse("# prose: off\nx = 1\n# prose: on\n");

        assert!(pipeline.unsettled(&source).is_empty());
    }

    #[test]
    fn unsettled_skips_a_rule_whose_edits_splice_back_to_the_same_text() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("x".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-x"),
        })]);
        let source = parse("x = 1\n");

        assert!(pipeline.unsettled(&source).is_empty());
    }

    #[test]
    fn with_defaults_registers_enabled_rules() {
        let config = Config::default();
        let pipeline = Pipeline::with_defaults(&config);
        assert_eq!(pipeline.len(), Pipeline::known_ids().len());
    }

    #[test]
    fn with_defaults_respects_rule_toggles() {
        let disabled = Pipeline::known_ids()
            .iter()
            .map(|id| format!("{id} = false"))
            .join("\n");
        let config: Config = toml::from_str(&format!("[rules]\n{disabled}\n"))
            .expect("every registered slug parses as a rule toggle");

        assert!(Pipeline::with_defaults(&config).is_empty());
    }

    #[test]
    fn with_filters_ignore_subtracts_from_configured_set() {
        let ignore = [AlignEquals::SLUG, AlphabetizeSiblings::SLUG];
        let pipeline = Pipeline::with_filters(&Config::default(), &[], &ignore);
        let slugs = registered_slugs(&pipeline);
        assert_eq!(slugs.len(), Pipeline::known_ids().len() - ignore.len());
        assert!(!slugs.contains(&AlignEquals::SLUG.as_str()));
        assert!(!slugs.contains(&AlphabetizeSiblings::SLUG.as_str()));
    }

    #[test]
    fn with_filters_select_minus_ignore_drops_overlap() {
        let pipeline = Pipeline::with_filters(
            &Config::default(),
            &[AlignEquals::SLUG, AlignColons::SLUG],
            &[AlignEquals::SLUG],
        );
        assert_eq!(registered_slugs(&pipeline), ["align-colons"]);
    }

    #[test]
    fn with_filters_select_overrides_disabled_config() {
        let mut config = Config::default();
        config.rules.align_equals.enabled = false;

        let pipeline = Pipeline::with_filters(&config, &[AlignEquals::SLUG], &[]);
        assert_eq!(registered_slugs(&pipeline), ["align-equals"]);
    }

    #[test]
    fn with_filters_select_with_default_config_restricts_to_listed_rules() {
        let pipeline = Pipeline::with_filters(&Config::default(), &[AlignEquals::SLUG], &[]);
        assert_eq!(registered_slugs(&pipeline), ["align-equals"]);
    }
}

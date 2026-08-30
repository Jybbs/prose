//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>` and a `Vec<TextRange>` of lint
//! ranges. The pipeline sorts and applies the edits into a fresh
//! buffer, then reparses and confirms the result still compiles before
//! handing the new `Source` to the next rule. Registration order follows
//! the data dependency, seating every rule that mutates a line's width,
//! a group's member order, or a statement's position ahead of every rule
//! that reads one. The settle check re-applies the enabled rules to a
//! completed run's output and names every rule still editing it.

use std::ops::Range;

use ruff_diagnostics::{Edit, SourceMap};
use ruff_python_ast::PythonVersion;
use ruff_text_size::Ranged;

use crate::{
    diagnostics::Diagnostic,
    primitives::edit::{apply_edits, apply_edits_mapped},
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

    /// Folds each rule's edits into `source` in registration order
    /// across `seats`, reparsing between rules and extending
    /// `diagnostics` with each rule's format findings when the caller
    /// supplies one.
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
    /// [`reparse_or_reject`].
    fn fold_rules(
        &self,
        source: Source,
        mut diagnostics: Option<&mut Vec<Diagnostic>>,
        seats: Range<usize>,
    ) -> Result<Source, PipelineError> {
        let gate = compile_gate(&source, self.target_version);
        self.rules[seats].iter().try_fold(source, |source, rule| {
            let rule_id = rule.id();
            let Some((groups, new_text, map)) = woven_groups(&**rule, &source) else {
                return Ok(source);
            };
            debug_assert!(
                new_text != source.text(),
                "rule `{rule_id}` emitted edits that produced identical text",
            );
            if let Some(collected) = diagnostics.as_deref_mut() {
                collected.extend(format_diagnostics(&**rule, groups));
            }
            reparse_or_reject(&source, new_text, rule_id, map, gate)
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

    /// Collects every rule's diagnostics against `source` without
    /// applying edits or reparsing between rules, so each range stays
    /// valid against the original buffer. Format rules contribute one
    /// diagnostic per surviving fix group and lint rules their lint
    /// diagnostics, both filtered through the suppression map exactly as
    /// [`run`](Self::run) filters them.
    pub fn diagnose(&self, source: &Source) -> Vec<Diagnostic> {
        self.diagnosed(source).0
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

    /// A rendering of every rule's settings, equal for two pipelines
    /// whose rules were constructed against selections they read
    /// alike.
    pub fn fingerprint(&self) -> String {
        format!("{:?}", self.rules)
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
            let groups = prepared_groups(&**rule, source);
            if groups.is_empty() {
                continue;
            }
            let rule_id = rule.id();
            match weave_distinct(&**rule, source, &groups) {
                Some((text, _)) if text != source.text() => {
                    report.editing.push(rule_id);
                    report.witness.get_or_insert((rule_id, text));
                }
                _ => report.unlanded.push(rule_id),
            }
        }
        report
    }

    /// One pipeline per rule this pipeline carries, in order, each
    /// holding its rule as this pipeline constructed it, so a rule that
    /// reads a sibling's flag keeps the answer this selection gave it.
    pub fn split(self) -> Vec<(RuleId, Self)> {
        let target_version = self.target_version;
        self.rules
            .into_iter()
            .map(|rule| {
                (
                    rule.id(),
                    Self {
                        rules: vec![rule],
                        target_version,
                    },
                )
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

/// Splices a rule's concatenated edits into `source`, returning the
/// woven text and, for a notebook, the `SourceMap` of cell-offset
/// deltas. An ordinary module skips the map.
fn weave_groups(source: &Source, edits: Vec<Edit>) -> Option<(String, Option<SourceMap>)> {
    if source.is_notebook() {
        apply_edits_mapped(source.text(), edits).map(|(text, map)| (text, Some(map)))
    } else {
        apply_edits(source.text(), edits).map(|text| (text, None))
    }
}

/// Weaves `rule`'s `groups` into `source`, checking first that no two
/// of its edits repeat.
fn weave_distinct(
    rule: &dyn Rule,
    source: &Source,
    groups: &[Vec<Edit>],
) -> Option<(String, Option<SourceMap>)> {
    debug_assert!(
        distinct_edits(groups),
        "rule `{}` emitted a duplicate edit, the signature of a walk reaching one node twice",
        rule.id(),
    );
    weave_groups(source, groups.concat())
}

/// Applies `rule` to `source` and weaves its surviving fix groups into
/// new text, returning `None` when no group survives or the edits do not
/// apply.
fn woven_groups(
    rule: &dyn Rule,
    source: &Source,
) -> Option<(Vec<Vec<Edit>>, String, Option<SourceMap>)> {
    let groups = prepared_groups(rule, source);
    if groups.is_empty() {
        return None;
    }
    let (new_text, map) = weave_distinct(rule, source, &groups)?;
    Some((groups, new_text, map))
}

#[cfg(test)]
mod tests {
    use std::{
        assert_matches,
        sync::{Arc, Mutex},
    };

    use itertools::Itertools;
    use rstest::rstest;
    use ruff_diagnostics::Edit;
    use ruff_text_size::{TextRange, TextSize};

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
            FUTURE_LEAD, GroupSentinelRule, assert_send_sync, breaks_compile, breaks_parse,
            never_settles, notebook, parse, range, self_overlapping,
        },
    };

    /// Test-only lint-only rule that returns the range list supplied
    /// at construction and never produces edits.
    #[derive(Debug)]
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
    #[derive(Debug)]
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
    #[derive(Debug)]
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
    #[derive(Debug)]
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
    fn downstream_rule_apply_sees_upstream_rewritten_text() {
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
        let source = parse("x = 1\n");

        pipeline.run(source).expect("both stages succeed");

        assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "y = 1\n"]);
    }

    #[test]
    fn empty_pipeline_returns_identical_source() {
        let pipeline = Pipeline::from_rules(Vec::new());
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline.run(source).expect("identity run succeeds");

        assert_eq!(result.text(), "x = 1\n");
        assert!(diagnostics.is_empty());
    }

    #[rstest]
    #[case("band-constants", "group-imports", false)]
    #[case("align-equals", "align-colons", true)]
    fn fingerprint_reads_a_sibling_flag_off_the_selection(
        #[case] rule: &'static str,
        #[case] sibling: &'static str,
        #[case] alike: bool,
    ) {
        let config = Config::default();
        let rule = RuleId::from(rule);
        let alone = Pipeline::with_filters(&config, &[rule], &[]);
        let beside = Pipeline::with_filters(&config, &[rule, RuleId::from(sibling)], &[]);
        let (_, seated) = beside
            .split()
            .into_iter()
            .find(|(id, _)| *id == rule)
            .expect("the pair seats the rule");

        assert_eq!(alone.fingerprint() == seated.fingerprint(), alike);
    }

    #[test]
    fn format_matches_the_text_half_of_run() {
        let pipeline = Pipeline::with_defaults(&Config::default());
        let text = "import sys\nimport os\n\n\n\n\nx  =  1\n";
        let formatted = pipeline.format(text.parse().unwrap()).unwrap();
        let (ran, _) = pipeline.run(text.parse().unwrap()).unwrap();
        assert_eq!(formatted.text(), ran.text());
    }

    #[test]
    fn format_span_segments_compose_to_the_full_fold() {
        let pipeline = Pipeline::with_defaults(&Config::default());
        let text = "import sys\nimport os\n\n\n\n\ny  =  2\n";
        let source: Source = text.parse().unwrap();
        let copy = source.clone();
        let full = pipeline.format(source).unwrap();
        let head = pipeline.format_span(copy, 0..20).unwrap();
        let tail = pipeline.format_span(head, 20..pipeline.len()).unwrap();
        assert_eq!(tail.text(), full.text());
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
    fn settle_report_holds_the_first_editing_rule_as_its_witness() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(never_settles("first")),
            Box::new(never_settles("second")),
        ]);
        let source = parse("x = 1\n");

        let report = pipeline.settle_report(&source);

        assert_eq!(
            report.editing,
            vec![RuleId::from("first"), RuleId::from("second")]
        );
        assert_matches!(report.witness, Some((id, _)) if id == RuleId::from("first"));
    }

    #[test]
    fn settle_report_names_a_rule_whose_fix_never_lands() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(self_overlapping()),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("x".to_owned(), range(0, 1))]],
                id: RuleId::from("rewrite-x-to-x"),
            }),
        ]);
        let source = parse("x = 1\n");

        let report = pipeline.settle_report(&source);

        assert!(report.editing.is_empty());
        assert_eq!(
            report.unlanded,
            vec![self_overlapping().id(), RuleId::from("rewrite-x-to-x")]
        );
        assert!(report.witness.is_none());
    }

    #[test]
    fn settle_report_names_the_editing_rule_and_the_text_it_weaves() {
        let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
        let source = parse("x = 1\n");

        let report = pipeline.settle_report(&source);

        assert_eq!(report.editing, vec![RuleId::from("widener")]);
        assert!(report.unlanded.is_empty());
        assert_matches!(
            report.witness,
            Some((id, text)) if id == RuleId::from("widener") && text == "yy = 1\n"
        );
    }

    #[test]
    fn split_seats_each_rule_alone_in_pipeline_order() {
        let config = Config::default();
        let selected = ["align-equals", "band-constants", "align-colons"].map(RuleId::from);
        let singles = Pipeline::with_filters(&config, &selected, &[]).split();

        assert_eq!(
            singles.iter().map(|(id, _)| id.as_str()).collect_vec(),
            ["band-constants", "align-colons", "align-equals"],
        );
        assert!(
            singles
                .iter()
                .all(|(id, single)| registered_slugs(single) == [id.as_str()])
        );
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

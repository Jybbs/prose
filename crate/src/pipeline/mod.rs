//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>` and a `Vec<TextRange>` of lint
//! ranges. The pipeline sorts and applies the edits into a fresh
//! buffer, then reparses before handing the new `Source` to the next
//! rule. Alignment rules run last so earlier rewrites settle before
//! padding widths are computed.

use crate::{
    diagnostics::Diagnostic,
    primitives::edit::apply_edits,
    rule::{Rule, RuleId},
    source::Source,
};

mod error;
mod filter;

pub use error::PipelineError;
use error::reparse_or_reject;
use filter::{drop_suppressed_lints, prepared_groups, unsuppressed_lints};

/// Ordered sequence of enabled rules, run against each source file.
pub struct Pipeline {
    rules: Vec<Box<dyn Rule>>,
}

impl Pipeline {
    /// Constructs a pipeline that performs no rewrites.
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    pub(crate) fn from_rules(rules: Vec<Box<dyn Rule>>) -> Self {
        Self { rules }
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
        let suppression = source.suppression_map();
        if suppression.file_is_suppressed() {
            return Vec::new();
        }
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            let rule_id = rule.id();
            let groups = prepared_groups(&**rule, source, suppression, rule_id);
            let message = rule.message();
            diagnostics.extend(
                groups
                    .into_iter()
                    .map(|group| Diagnostic::format(rule_id, group, message.to_owned())),
            );
            diagnostics.extend(unsuppressed_lints(&**rule, source, suppression));
        }
        drop_suppressed_lints(&mut diagnostics, source, suppression);
        diagnostics
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
    /// File-level `# prose: off` short-circuits to identity. The
    /// suppression map otherwise filters each fix group's edits per-rule
    /// (off spans plus `# prose: skip[<id>]`), drops a group left empty,
    /// and filters lint diagnostics per-line (`# prose: ignore`).
    /// Alignment rules pre-exclude suppressed rows before grouping, so
    /// this edit-level pass is a no-op for them.
    ///
    /// # Errors
    ///
    /// Returns `PipelineError::Reparse` when a rule's edit list
    /// produces text that does not re-parse as Python.
    pub fn run(&self, source: Source) -> Result<(Source, Vec<Diagnostic>), PipelineError> {
        if source.suppression_map().file_is_suppressed() {
            return Ok((source, Vec::new()));
        }
        let (source, mut diagnostics) = self.rules.iter().try_fold(
            (source, Vec::new()),
            |(source, mut diagnostics), rule| {
                let suppression = source.suppression_map();
                let rule_id = rule.id();
                let groups = prepared_groups(&**rule, &source, suppression, rule_id);
                diagnostics.extend(unsuppressed_lints(&**rule, &source, suppression));
                if groups.is_empty() {
                    return Ok((source, diagnostics));
                }
                let message = rule.message();
                let Some(new_text) = apply_edits(source.text(), groups.concat()) else {
                    return Ok((source, diagnostics));
                };
                debug_assert!(
                    new_text != source.text(),
                    "rule `{rule_id}` emitted edits that produced identical text",
                );
                diagnostics.extend(
                    groups
                        .into_iter()
                        .map(|group| Diagnostic::format(rule_id, group, message.to_owned())),
                );
                reparse_or_reject(&source, new_text, rule_id).map(|src| (src, diagnostics))
            },
        )?;
        drop_suppressed_lints(&mut diagnostics, &source, source.suppression_map());
        Ok((source, diagnostics))
    }

    /// Replays the editing rules to surface a rule whose output fails to
    /// re-parse, discarding the rewritten text and the diagnostics
    /// [`run`](Self::run) would build. `check` calls this when
    /// [`diagnose`](Self::diagnose) flags format work, in place of the
    /// full `run`.
    ///
    /// # Errors
    ///
    /// Returns `PipelineError::Reparse` when a rule's edit list produces
    /// text that does not re-parse as Python.
    pub(crate) fn validate(&self, source: Source) -> Result<(), PipelineError> {
        self.rules
            .iter()
            .try_fold(source, |source, rule| {
                let rule_id = rule.id();
                let groups = prepared_groups(&**rule, &source, source.suppression_map(), rule_id);
                if groups.is_empty() {
                    return Ok(source);
                }
                let Some(new_text) = apply_edits(source.text(), groups.concat()) else {
                    return Ok(source);
                };
                reparse_or_reject(&source, new_text, rule_id)
            })
            .map(drop)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use assert_matches::assert_matches;
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::config::Config;
    use crate::diagnostics::Severity;
    use crate::primitives::edit::singleton_groups;
    use crate::testing::{GroupSentinelRule, assert_send_sync, breaks_parse, parse, range};

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
            self.seen.lock().unwrap().push(source.text().to_owned());
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
            .find(|d| d.severity == Severity::Format)
            .expect("format finding");
        assert_eq!(format.rule.as_str(), "rewrite-x-to-y");
        let lint = diagnostics
            .iter()
            .find(|d| d.severity == Severity::Lint)
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
    fn diagnose_drops_findings_under_a_suppressed_span() {
        let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
            id: RuleId::from("flag-stuff"),
            ranges: vec![range(13, 14)],
        })]);
        let source = parse("# prose: off\nx = 1\n");

        assert!(pipeline.diagnose(&source).is_empty());
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

        assert_eq!(*seen.lock().unwrap(), ["x = 1\n", "y = 1\n"]);
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

        let err = pipeline.run(source).expect_err("reparse should fail");

        match err {
            PipelineError::Reparse { rule, .. } => assert_eq!(rule.as_str(), "breaks-parse"),
        }
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

        assert_eq!(*log.lock().unwrap(), ["first", "second", "third"]);
    }

    #[test]
    fn run_declines_an_overlapping_group_as_a_no_op() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![
                Edit::range_replacement("Y".to_owned(), range(0, 3)),
                Edit::range_replacement("Z".to_owned(), range(2, 5)),
            ]],
            id: RuleId::from("self-overlapping"),
        })]);
        let source = parse("x = 1\n");

        let (result, diagnostics) = pipeline
            .run(source)
            .expect("overlap degrades, run continues");

        assert_eq!(result.text(), "x = 1\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn run_drops_edits_whose_range_overlaps_a_suppressed_span() {
        // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
        //         |0--------|11----|17--------|27----|33
        // Edit at 11..16 (`x = 1`) sits inside the suppressed
        // [0..17) span and must be dropped, leaving the unsuppressed
        // edit at 27..32 (`z = 9`) to apply.
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
    fn run_drops_only_the_suppressed_edit_within_a_group() {
        // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
        //         |0--------|11----|17--------|27----|33
        // The group bundles an edit at 11..16 (inside the suppressed
        // [0..17) span) with one at 27..32. Per-edit filtering drops
        // only the suppressed edit, leaving the survivor to apply as a
        // single-edit fix.
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![
                Edit::range_replacement("y".to_owned(), range(11, 16)),
                Edit::range_replacement("Z".to_owned(), range(27, 32)),
            ]],
            id: RuleId::from("rewrite-x-and-z"),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

        let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nZ\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("survivor fix")
                .edits()
                .len(),
            1
        );
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
        assert!(log.lock().unwrap().is_empty());
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
    fn validate_passes_a_clean_rewrite() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("y".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-y"),
        })]);
        let source = parse("x = 1\n");

        assert!(pipeline.validate(source).is_ok());
    }

    #[test]
    fn validate_passes_when_no_rule_edits() {
        let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
            groups: vec![Vec::new()],
            id: RuleId::from("emits-empty-group"),
        })]);
        let source = parse("x = 1\n");

        assert!(pipeline.validate(source).is_ok());
    }

    #[test]
    fn validate_surfaces_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        assert_matches!(
            pipeline.validate(source),
            Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "breaks-parse"
        );
    }

    #[test]
    fn with_defaults_registers_enabled_rules() {
        let config = Config::default();
        let pipeline = Pipeline::with_defaults(&config);
        assert_eq!(pipeline.len(), Pipeline::known_ids().len());
    }

    #[test]
    fn with_defaults_respects_rule_toggles() {
        let mut config = Config::default();
        config.rules.align_colons.enabled = false;
        config.rules.align_comparisons.enabled = false;
        config.rules.align_equals.enabled = false;
        config.rules.align_imports.enabled = false;
        config.rules.align_match_case.enabled = false;
        config.rules.alphabetize.enabled = false;
        config.rules.bare_imports.enabled = false;
        config.rules.blank_lines.enabled = false;
        config.rules.call_layout.enabled = false;
        config.rules.collection_layout.enabled = false;
        config.rules.docstring_expand.enabled = false;
        config.rules.docstring_frame.enabled = false;
        config.rules.docstring_wrap.enabled = false;
        config.rules.import_layout.enabled = false;
        config.rules.legacy_union_syntax.enabled = false;
        config.rules.reassigned_constants.enabled = false;
        config.rules.shed_parentheses.enabled = false;
        config.rules.signature_annotations.enabled = false;
        config.rules.signature_layout.enabled = false;
        config.rules.single_use_variables.enabled = false;
        config.rules.step_narration.enabled = false;
        config.rules.strip_align_padding.enabled = false;
        config.rules.strip_none_return.enabled = false;
        config.rules.strip_trailing_commas.enabled = false;
        config.rules.unsorted_parameters.enabled = false;
        config.rules.unused_future_annotations.enabled = false;
        let pipeline = Pipeline::with_defaults(&config);
        assert!(pipeline.is_empty());
    }

    #[test]
    fn with_filters_ignore_subtracts_from_configured_set() {
        let ignore = [RuleId::from("align-equals"), RuleId::from("alphabetize")];
        let pipeline = Pipeline::with_filters(&Config::default(), &[], &ignore);
        let slugs = registered_slugs(&pipeline);
        assert_eq!(slugs.len(), Pipeline::known_ids().len() - ignore.len());
        assert!(!slugs.contains(&"align-equals"));
        assert!(!slugs.contains(&"alphabetize"));
    }

    #[test]
    fn with_filters_select_minus_ignore_drops_overlap() {
        let pipeline = Pipeline::with_filters(
            &Config::default(),
            &[RuleId::from("align-equals"), RuleId::from("align-colons")],
            &[RuleId::from("align-equals")],
        );
        assert_eq!(registered_slugs(&pipeline), ["align-colons"]);
    }

    #[test]
    fn with_filters_select_overrides_disabled_config() {
        let mut config = Config::default();
        config.rules.align_equals.enabled = false;

        let pipeline = Pipeline::with_filters(&config, &[RuleId::from("align-equals")], &[]);
        assert_eq!(registered_slugs(&pipeline), ["align-equals"]);
    }

    #[test]
    fn with_filters_select_with_default_config_restricts_to_listed_rules() {
        let pipeline =
            Pipeline::with_filters(&Config::default(), &[RuleId::from("align-equals")], &[]);
        assert_eq!(registered_slugs(&pipeline), ["align-equals"]);
    }
}

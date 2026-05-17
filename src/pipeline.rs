//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>` and a `Vec<TextRange>` of lint
//! ranges. The pipeline sorts and applies the edits into a fresh
//! buffer, then reparses before handing the new `Source` to the next
//! rule. Alignment rules run last so earlier rewrites settle before
//! padding widths are computed.

use ruff_python_parser::ParseError;
use thiserror::Error;

use crate::diagnostics::{Diagnostic, Severity};
use crate::primitives::edit::apply_edits;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

/// Ordered sequence of enabled rules, run against each source file.
///
/// Use [`Pipeline::with_defaults`] to build one from a loaded
/// [`crate::config::Config`], [`Pipeline::for_rule`] to register
/// exactly one rule by name, or [`Pipeline::empty`] for a pipeline
/// with no rules.
pub struct Pipeline {
    rules: Vec<Box<dyn Rule>>,
}

impl Pipeline {
    /// Constructs a pipeline that performs no rewrites. Useful for
    /// callers that need a `Pipeline` value but no rules to run.
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

    /// Returns every registered rule's id in a stable order. Useful
    /// for CLI `--select` / `--ignore` validation and for rule
    /// listings. Surfaces the same registry that
    /// [`RuleId::from_str`](crate::rule::RuleId) consults.
    pub fn known_ids() -> &'static [RuleId] {
        crate::rule::KNOWN_IDS
    }

    /// Runs each registered rule against `source` in order and
    /// returns the rewritten source paired with the diagnostics each
    /// rule emitted.
    ///
    /// Per rule, the pipeline calls `apply`, drops edits inside any
    /// `# fmt: off` span via the `SuppressionMap`, derives one
    /// `Severity::Format` `Diagnostic` per surviving edit, collects
    /// the `Severity::Lint` diagnostics `Rule::lint` returned through
    /// the same fmt-suppression filter, then splices the edits into
    /// the current text and reparses for the next rule. After the
    /// rule chain finishes, every `Severity::Lint` diagnostic whose
    /// line carries a matching `# prose: ignore` directive is
    /// dropped. An empty pipeline collapses to the identity transform
    /// and returns no diagnostics.
    ///
    /// # Errors
    ///
    /// Returns `PipelineError::Reparse` when a rule's edit list
    /// produces text that does not re-parse as Python. This surfaces
    /// rule bugs rather than silently swallowing them.
    pub fn run(&self, source: Source) -> Result<(Source, Vec<Diagnostic>), PipelineError> {
        let (source, mut diagnostics) = self.rules.iter().try_fold(
            (source, Vec::new()),
            |(source, mut diagnostics), rule| {
                let mut edits = rule.apply(&source);
                let suppression = source.suppression_map();
                if suppression.has_format_suppression() {
                    edits.retain(|edit| !suppression.intersects(edit));
                }
                diagnostics.extend(
                    rule.lint(&source)
                        .into_iter()
                        .filter(|d| !suppression.intersects(d.range)),
                );
                if edits.is_empty() {
                    return Ok((source, diagnostics));
                }
                let rule_id = rule.id();
                let message = rule.message();
                diagnostics.extend(
                    edits
                        .iter()
                        .map(|edit| Diagnostic::format(rule_id, edit.clone(), message.to_owned())),
                );
                let new_text = apply_edits(source.text(), edits);
                debug_assert!(
                    new_text != source.text(),
                    "rule `{}` emitted edits that produced identical text",
                    rule.id(),
                );
                source
                    .reparse(new_text)
                    .map(|src| (src, diagnostics))
                    .map_err(|source| PipelineError::Reparse {
                        rule: rule.id(),
                        source,
                    })
            },
        )?;
        let suppression = source.suppression_map();
        if suppression.has_lint_suppression() {
            diagnostics.retain(|d| {
                d.severity != Severity::Lint
                    || !suppression
                        .is_lint_suppressed_at(source.line_index(d.range.start()), d.rule)
            });
        }
        Ok((source, diagnostics))
    }
}

/// Failure modes surfaced by the pipeline itself.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("rule `{rule}` produced output that did not parse")]
    Reparse {
        rule: RuleId,
        #[source]
        source: ParseError,
    },
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::config::Config;
    use crate::test_support::{assert_send_sync, parse, range};

    /// Test-only rule that records its own id into a shared log and
    /// returns the edit list supplied at construction time.
    struct SentinelRule {
        edits: Vec<Edit>,
        id: RuleId,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Rule for SentinelRule {
        fn apply(&self, _source: &Source) -> Vec<Edit> {
            self.log.lock().expect("log mutex").push(self.id.as_str());
            self.edits.clone()
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn message(&self) -> &'static str {
            "test rule"
        }
    }

    /// Test-only lint-only rule that returns the range list supplied
    /// at construction and never produces edits.
    struct LintSentinelRule {
        id: RuleId,
        ranges: Vec<TextRange>,
    }

    impl Rule for LintSentinelRule {
        fn apply(&self, _source: &Source) -> Vec<Edit> {
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

    /// Test-only rule that captures `source.text()` at apply time and
    /// returns the edit list supplied at construction.
    struct TextCapturingRule {
        edits: Vec<Edit>,
        id: RuleId,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl Rule for TextCapturingRule {
        fn apply(&self, source: &Source) -> Vec<Edit> {
            self.seen.lock().unwrap().push(source.text().to_owned());
            self.edits.clone()
        }

        fn id(&self) -> RuleId {
            self.id
        }

        fn message(&self) -> &'static str {
            "test rule"
        }
    }

    fn registered_slugs(pipeline: &Pipeline) -> Vec<&'static str> {
        pipeline.rules.iter().map(|r| r.id().as_str()).collect()
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
        let mut registered: Vec<&'static str> =
            pipeline.rules.iter().map(|r| r.id().as_str()).collect();
        registered.sort_unstable();
        let mut known: Vec<&'static str> =
            Pipeline::known_ids().iter().map(|id| id.as_str()).collect();
        known.sort_unstable();
        assert_eq!(registered, known);
    }

    #[test]
    fn pipeline_is_send_and_sync() {
        assert_send_sync::<Pipeline>();
    }

    #[test]
    fn reparse_failure_surfaces_rule_id() {
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            edits: vec![Edit::range_replacement("def foo(".to_owned(), range(0, 5))],
            id: RuleId::from("breaks-parse"),
            log: log.clone(),
        })]);
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
                edits: Vec::new(),
                id: RuleId::from("first"),
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                edits: Vec::new(),
                id: RuleId::from("second"),
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                edits: Vec::new(),
                id: RuleId::from("third"),
                log: log.clone(),
            }),
        ]);
        let source = parse("x = 1\n");

        pipeline.run(source).expect("all rules succeed");

        assert_eq!(*log.lock().unwrap(), ["first", "second", "third"]);
    }

    #[test]
    fn run_drops_edits_whose_range_overlaps_a_suppressed_span() {
        // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
        //         |0--------|11----|17--------|27----|33
        // Edit at 11..16 (`x = 1`) sits inside the suppressed
        // [0..17) span and must be dropped, leaving the unsuppressed
        // edit at 27..32 (`z = 9`) to apply.
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            edits: vec![
                Edit::range_replacement("y".to_owned(), range(11, 16)),
                Edit::range_replacement("Z".to_owned(), range(27, 32)),
            ],
            id: RuleId::from("rewrite-x-and-z"),
            log: Arc::new(Mutex::new(Vec::new())),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

        let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nZ\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-and-z");
    }

    #[test]
    fn run_emits_one_diagnostic_per_surviving_edit() {
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
            id: RuleId::from("rewrite-x-to-y"),
            log: Arc::new(Mutex::new(Vec::new())),
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
    fn run_skips_reparse_when_every_edit_is_suppressed() {
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            edits: vec![Edit::range_replacement("y".to_owned(), range(11, 16))],
            id: RuleId::from("rewrite-x-to-y"),
            log: Arc::new(Mutex::new(Vec::new())),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\n");

        let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\n");
        assert!(diagnostics.is_empty());
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
        config.rules.align_equals.enabled = false;
        config.rules.align_imports.enabled = false;
        config.rules.alphabetize.enabled = false;
        config.rules.bare_import_allowlist.enabled = false;
        config.rules.blank_lines.enabled = false;
        config.rules.collection_layout.enabled = false;
        config.rules.docstring_wrap.enabled = false;
        config.rules.legacy_union_syntax.enabled = false;
        config.rules.loose_constants.enabled = false;
        config.rules.match_case_align.enabled = false;
        config.rules.multi_line_docstrings.enabled = false;
        config.rules.no_single_line_docstrings.enabled = false;
        config.rules.no_step_narration.enabled = false;
        config.rules.signature_layout.enabled = false;
        config.rules.single_use_variables.enabled = false;
        config.rules.singleton_rule.enabled = false;
        config.rules.strip_trailing_commas.enabled = false;
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
        config.rules.align_colons.enabled = false;
        config.rules.align_equals.enabled = false;
        config.rules.align_imports.enabled = false;
        config.rules.alphabetize.enabled = false;
        config.rules.bare_import_allowlist.enabled = false;
        config.rules.blank_lines.enabled = false;
        config.rules.collection_layout.enabled = false;
        config.rules.legacy_union_syntax.enabled = false;
        config.rules.loose_constants.enabled = false;
        config.rules.match_case_align.enabled = false;
        config.rules.no_step_narration.enabled = false;
        config.rules.signature_layout.enabled = false;
        config.rules.single_use_variables.enabled = false;
        config.rules.singleton_rule.enabled = false;
        config.rules.strip_trailing_commas.enabled = false;
        config.rules.unused_future_annotations.enabled = false;

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

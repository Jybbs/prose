//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>`. The pipeline sorts and applies the
//! edits into a fresh buffer, then reparses before handing the new
//! `Source` to the next rule. Alignment rules run last so earlier
//! rewrites settle before padding widths are computed.

use ruff_python_parser::ParseError;
use thiserror::Error;

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

    /// Runs each registered rule against `source` in order and reports
    /// whether the pipeline changed the text.
    ///
    /// Per rule, the pipeline calls `apply`, splices the returned edits
    /// into the current text, reparses into a fresh `Source`, and hands
    /// that to the next rule. An empty pipeline collapses to the
    /// identity transform and reports `changed = false`.
    ///
    /// # Errors
    ///
    /// Returns `PipelineError::Reparse` when a rule's edit list
    /// produces text that does not re-parse as Python. This surfaces
    /// rule bugs rather than silently swallowing them.
    pub fn run(&self, source: Source) -> Result<(Source, bool), PipelineError> {
        self.rules
            .iter()
            .try_fold((source, false), |(source, changed), rule| {
                let mut edits = rule.apply(&source);
                let suppression = source.suppression_map();
                if !suppression.is_empty() {
                    edits.retain(|edit| !suppression.intersects(edit));
                }
                if edits.is_empty() {
                    return Ok((source, changed));
                }
                let new_text = apply_edits(source.text(), edits);
                debug_assert!(
                    new_text != source.text(),
                    "rule `{}` emitted edits that produced identical text",
                    rule.id(),
                );
                source
                    .reparse(new_text)
                    .map(|src| (src, true))
                    .map_err(|source| PipelineError::Reparse {
                        rule: rule.id(),
                        source,
                    })
            })
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

        let (result, changed) = pipeline.run(source).expect("identity run succeeds");

        assert_eq!(result.text(), "x = 1\n");
        assert!(!changed);
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

        let (result, changed) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nZ\n");
        assert!(changed);
    }

    #[test]
    fn run_reports_changed_when_a_rule_rewrites_text() {
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
            id: RuleId::from("rewrite-x-to-y"),
            log: Arc::new(Mutex::new(Vec::new())),
        })]);
        let source = parse("x = 1\n");

        let (result, changed) = pipeline.run(source).expect("rewrite succeeds");

        assert_eq!(result.text(), "y = 1\n");
        assert!(changed);
    }

    #[test]
    fn run_skips_reparse_when_every_edit_is_suppressed() {
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            edits: vec![Edit::range_replacement("y".to_owned(), range(11, 16))],
            id: RuleId::from("rewrite-x-to-y"),
            log: Arc::new(Mutex::new(Vec::new())),
        })]);
        let source = parse("# fmt: off\nx = 1\n# fmt: on\n");

        let (result, changed) = pipeline.run(source).expect("filtered run succeeds");

        assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\n");
        assert!(!changed);
    }

    #[test]
    fn with_defaults_registers_enabled_rules() {
        let config = Config::default();
        let pipeline = Pipeline::with_defaults(&config);
        assert_eq!(pipeline.len(), 8);
    }

    #[test]
    fn with_defaults_respects_rule_toggles() {
        let mut config = Config::default();
        config.rules.align_colons.enabled = false;
        config.rules.align_equals.enabled = false;
        config.rules.align_imports.enabled = false;
        config.rules.alphabetize.enabled = false;
        config.rules.collection_layout.enabled = false;
        config.rules.match_case_align.enabled = false;
        config.rules.singleton_rule.enabled = false;
        config.rules.strip_trailing_commas.enabled = false;
        let pipeline = Pipeline::with_defaults(&config);
        assert!(pipeline.is_empty());
    }
}

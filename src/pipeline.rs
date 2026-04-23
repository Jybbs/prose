//! Runs the enabled rules against a source file in deterministic order.
//!
//! Each rule returns a `Vec<Edit>`. The pipeline sorts and applies the
//! edits into a fresh buffer, then reparses before handing the new
//! `Source` to the next rule. Alignment rules run last so earlier
//! rewrites settle before padding widths are computed.

use ruff_diagnostics::Edit;
use ruff_python_parser::ParseError;
use ruff_text_size::Ranged;
use thiserror::Error;

use crate::config::Config;
use crate::source::Source;

/// Every rule in `prose` implements this trait and nothing more.
///
/// Implementations inspect `source` and return the edits that would
/// bring it into conformance. An empty `Vec<Edit>` means the rule has
/// nothing to say, and the pipeline skips the reparse for that rule.
///
/// Rules must be `Send + Sync` so that the pipeline can run across
/// files in parallel without moving the rule list per worker.
pub trait Rule: Send + Sync {
    /// Stable identifier matching the rule's `[tool.prose.rules]` key
    /// (kebab-case, e.g. `"align-equals"`). Surfaces in diagnostic
    /// output when a rule produces unparseable text.
    fn name(&self) -> &'static str;

    /// Computes the edit list this rule would apply to `source`.
    ///
    /// Edits must not overlap after sorting. The pipeline's applicator
    /// debug-asserts this invariant, which is a rule-authoring bug if
    /// it ever fires.
    ///
    /// Returns a bare `Vec<Edit>` rather than `ruff_diagnostics::Fix`
    /// because prose has no concept of `Applicability` or
    /// `IsolationLevel` yet. The pipeline sorts the list itself and
    /// wraps `Fix` only if a future rule needs those annotations.
    fn apply(&self, source: &Source) -> Vec<Edit>;
}

/// Ordered sequence of enabled rules, run against each source file.
///
/// Use [`Pipeline::with_defaults`] to build one from a loaded [`Config`].
/// Construct directly with an empty rule list for tests that exercise
/// the identity path.
pub struct Pipeline {
    rules: Vec<Box<dyn Rule>>,
}

impl Pipeline {
    /// Builds a pipeline registering every rule enabled in `config`.
    ///
    /// Execution order: `one_per_line_collections` → `alphabetize` →
    /// `strip_trailing_commas` → `match_case_align` → `singleton_rule`
    /// → `align_imports` → `align_colons` → `align_equals`. Each rule
    /// PR adds one registration line at its ordered slot below.
    pub fn with_defaults(config: &Config) -> Self {
        let rules: Vec<Box<dyn Rule>> = Vec::new();
        let _ = config;
        // if config.rules.one_per_line_collections { rules.push(Box::new(OnePerLineCollections)); }
        // if config.rules.alphabetize { rules.push(Box::new(Alphabetize)); }
        // if config.rules.strip_trailing_commas { rules.push(Box::new(StripTrailingCommas)); }
        // if config.rules.match_case_align { rules.push(Box::new(MatchCaseAlign)); }
        // if config.rules.singleton_rule { rules.push(Box::new(SingletonRule)); }
        // if config.rules.align_imports { rules.push(Box::new(AlignImports)); }
        // if config.rules.align_colons { rules.push(Box::new(AlignColons)); }
        // if config.rules.align_equals { rules.push(Box::new(AlignEquals)); }
        Self { rules }
    }

    /// Constructs a pipeline from an explicit rule list.
    ///
    /// Primarily used by tests and by any future integration that
    /// wants a subset or a custom ordering outside `Config` control.
    pub fn from_rules(rules: Vec<Box<dyn Rule>>) -> Self {
        Self { rules }
    }

    /// Returns `true` when the pipeline has no rules registered.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Number of rules the pipeline would run.
    pub fn len(&self) -> usize {
        self.rules.len()
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
        let original = source.text().to_owned();
        let formatted = self.rules.iter().try_fold(source, |source, rule| {
            let edits = rule.apply(&source);
            if edits.is_empty() {
                return Ok(source);
            }
            let new_text = apply_edits(source.text(), edits);
            source
                .reparse(new_text)
                .map_err(|source| PipelineError::Reparse {
                    rule: rule.name(),
                    source,
                })
        })?;
        let changed = formatted.text() != original;
        Ok((formatted, changed))
    }
}

/// Failure modes surfaced by the pipeline itself.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("rule `{rule}` produced output that did not parse")]
    Reparse {
        rule: &'static str,
        #[source]
        source: ParseError,
    },
}

/// Splices `edits` into `text` and returns the resulting string.
///
/// Sorts edits by start-then-end (via `Edit`'s `Ord` impl), then walks
/// the list forward once, copying the unchanged spans between edits
/// into a pre-sized buffer and substituting each edit's replacement
/// at its position. Linear in the source length regardless of how
/// many edits apply. Debug builds assert the sorted edits are
/// non-overlapping, a rule-authoring invariant.
fn apply_edits(text: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_unstable();
    debug_assert!(
        edits.windows(2).all(|w| w[0].end() <= w[1].start()),
        "edits overlap"
    );
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for edit in edits {
        let start = edit.start().to_usize();
        let end = edit.end().to_usize();
        out.push_str(&text[cursor..start]);
        if let Some(content) = edit.content() {
            out.push_str(content);
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use ruff_text_size::TextRange;

    use super::*;

    /// Test-only rule that records its own name into a shared log and
    /// returns the edit list supplied at construction time.
    struct SentinelRule {
        name: &'static str,
        edits: Vec<Edit>,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Rule for SentinelRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn apply(&self, _source: &Source) -> Vec<Edit> {
            self.log.lock().expect("log mutex").push(self.name);
            self.edits.clone()
        }
    }

    /// Test-only rule that captures `source.text()` at apply time and
    /// returns the edit list supplied at construction.
    struct TextCapturingRule {
        name: &'static str,
        edits: Vec<Edit>,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl Rule for TextCapturingRule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn apply(&self, source: &Source) -> Vec<Edit> {
            self.seen.lock().unwrap().push(source.text().to_owned());
            self.edits.clone()
        }
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn apply_edits_handles_insertions_and_deletions() {
        let out = apply_edits(
            "abcd",
            vec![
                Edit::insertion("<".to_owned(), 0u32.into()),
                Edit::range_deletion(range(2, 3)),
            ],
        );

        assert_eq!(out, "<abd");
    }

    #[test]
    fn apply_edits_handles_multiple_non_overlapping_edits() {
        let out = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("X".to_owned(), range(0, 1)),
                Edit::range_replacement("Y".to_owned(), range(4, 5)),
            ],
        );

        assert_eq!(out, "XbcdYf");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "edits overlap")]
    fn apply_edits_panics_on_overlap_in_debug() {
        let _ = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("X".to_owned(), range(0, 3)),
                Edit::range_replacement("Y".to_owned(), range(2, 4)),
            ],
        );
    }

    #[test]
    fn apply_edits_sorts_unsorted_input() {
        let out = apply_edits(
            "abcdef",
            vec![
                Edit::range_replacement("Y".to_owned(), range(4, 5)),
                Edit::range_replacement("X".to_owned(), range(0, 1)),
            ],
        );

        assert_eq!(out, "XbcdYf");
    }

    #[test]
    fn downstream_rule_apply_sees_upstream_rewritten_text() {
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(TextCapturingRule {
                name: "rewrite-x-to-y",
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                seen: seen.clone(),
            }),
            Box::new(TextCapturingRule {
                name: "downstream-observer",
                edits: Vec::new(),
                seen: seen.clone(),
            }),
        ]);
        let source = Source::from_str("x = 1\n").expect("parses");

        pipeline.run(source).expect("both stages succeed");

        assert_eq!(*seen.lock().unwrap(), ["x = 1\n", "y = 1\n"]);
    }

    #[test]
    fn empty_pipeline_returns_identical_source() {
        let pipeline = Pipeline::from_rules(Vec::new());
        let source = Source::from_str("x = 1\n").expect("parses");

        let (result, changed) = pipeline.run(source).expect("identity run succeeds");

        assert_eq!(result.text(), "x = 1\n");
        assert!(!changed);
    }

    #[test]
    fn pipeline_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Pipeline>();
    }

    #[test]
    fn reparse_failure_surfaces_rule_name() {
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
            name: "breaks-parse",
            edits: vec![Edit::range_replacement("def foo(".to_owned(), range(0, 5))],
            log: log.clone(),
        })]);
        let source = Source::from_str("x = 1\n").expect("parses");

        let err = pipeline.run(source).expect_err("reparse should fail");

        match err {
            PipelineError::Reparse { rule, .. } => assert_eq!(rule, "breaks-parse"),
        }
    }

    #[test]
    fn rule_with_non_empty_edits_reparses_between_stages() {
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(SentinelRule {
                name: "rewrite-x-to-y",
                edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                name: "downstream",
                edits: Vec::new(),
                log: log.clone(),
            }),
        ]);
        let source = Source::from_str("x = 1\n").expect("parses");

        let (result, changed) = pipeline.run(source).expect("both stages succeed");

        assert_eq!(result.text(), "y = 1\n");
        assert!(changed);
        assert_eq!(*log.lock().unwrap(), ["rewrite-x-to-y", "downstream"]);
    }

    #[test]
    fn rules_run_in_registration_order() {
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let pipeline = Pipeline::from_rules(vec![
            Box::new(SentinelRule {
                name: "first",
                edits: Vec::new(),
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                name: "second",
                edits: Vec::new(),
                log: log.clone(),
            }),
            Box::new(SentinelRule {
                name: "third",
                edits: Vec::new(),
                log: log.clone(),
            }),
        ]);
        let source = Source::from_str("x = 1\n").expect("parses");

        pipeline.run(source).expect("all rules succeed");

        assert_eq!(*log.lock().unwrap(), ["first", "second", "third"]);
    }

    #[test]
    fn with_defaults_registers_no_rules_today() {
        let config = Config::default();
        let pipeline = Pipeline::with_defaults(&config);
        assert!(pipeline.is_empty());
    }
}

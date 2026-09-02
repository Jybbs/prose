//! The report over a rewrite whose settle check names rules.
//!
//! A run that rewrote a file re-applies its enabled rules to the output
//! it produced, a `format` run re-applying the rules that edited on the
//! first pass and `check --validate` every enabled rule. Where any
//! still edits, the rewrite is a defect in Prose rather than in the
//! file beneath it, and this module builds the record the CLI and the
//! language server both render: the narrowed rule subset that
//! reproduces it, the configuration the run resolved, the output it
//! wrote, and what a second pass turns that output into.

use std::collections::BTreeSet;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{config::Config, pipeline::Pipeline, rule::RuleId, source::Source};

mod form;
mod narrow;

pub(crate) use form::report_url;
use narrow::{editing_subset, reproducing_subset};

/// One file's rewrite that a second pass would change.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct UnstableRewrite {
    /// The keys the run's `[tool.prose]` table set away from the
    /// default, empty for a run on the defaults.
    pub(crate) config_toml: String,
    /// The output the run wrote.
    pub(crate) first: String,
    /// The narrowest subset reproducing the defect, the `--select`
    /// list the report names.
    pub(crate) rules: Vec<RuleId>,
    /// What a second pass turns `first` into.
    pub(crate) second: String,
}

impl UnstableRewrite {
    /// `None` where `config` turns the report off or the enabled rules
    /// leave `formatted` settled. The subset narrows to a rule alone or
    /// a rule pair where one reproduces, falling back to the whole
    /// selection the run carried, which a notebook takes outright.
    pub(crate) fn detect(
        pipeline: &Pipeline,
        config: &Config,
        original: &str,
        formatted: &Source,
    ) -> Option<Self> {
        let editing = pipeline.unsettled(formatted);
        Self::from_editing(pipeline, config, formatted, &editing, || {
            narrowed(pipeline, config, original, &editing)
        })
    }

    /// The report for a probe hit on the own-output ledger, where
    /// `original` is a prior run's own output that this run rewrote.
    /// The proof already happened, so the walk the eager path runs is
    /// skipped and the editing set reads straight off the marked bytes,
    /// narrowed to the smallest subset still editing them.
    pub(crate) fn detect_marked(
        pipeline: &Pipeline,
        config: &Config,
        original: &str,
        formatted: &Source,
    ) -> Option<Self> {
        let source = original.parse::<Source>().ok()?;
        let editing = pipeline.unsettled(&source);
        Self::from_editing(pipeline, config, formatted, &editing, || {
            editing_subset(&editing, &source, filtered(config))
        })
    }

    /// The report a `format` run or the editor makes over `formatted`,
    /// its output for `original`, re-applying through
    /// [`Pipeline::unsettled_among`] only `fired`, the rules that edited
    /// on the first pass. `None` where those rules leave `formatted`
    /// settled, a rule silent on the first pass being left to the full
    /// walk [`detect`](Self::detect) runs.
    pub(crate) fn detect_narrowed(
        pipeline: &Pipeline,
        config: &Config,
        original: &str,
        formatted: &Source,
        fired: &BTreeSet<RuleId>,
    ) -> Option<Self> {
        let editing = pipeline.unsettled_among(formatted, fired);
        Self::from_editing(pipeline, config, formatted, &editing, || {
            narrowed(pipeline, config, original, &editing)
        })
    }

    /// The report over `formatted` where `editing` names a rule, `None`
    /// where it is empty, under the subset `narrow` names or the whole
    /// selection where it names none.
    fn from_editing(
        pipeline: &Pipeline,
        config: &Config,
        formatted: &Source,
        editing: &[RuleId],
        narrow: impl FnOnce() -> Option<Vec<RuleId>>,
    ) -> Option<Self> {
        (!editing.is_empty()).then(|| {
            Self::over(
                pipeline,
                config,
                formatted,
                subset(pipeline, formatted, narrow),
            )
        })
    }

    /// The record over `formatted` under the reproducing `rules`.
    fn over(pipeline: &Pipeline, config: &Config, formatted: &Source, rules: Vec<RuleId>) -> Self {
        Self {
            config_toml: config.to_changed_toml(),
            first: formatted.text().to_owned(),
            rules,
            second: second_pass(pipeline, formatted),
        }
    }

    /// A subject naming no `path` takes the `-` stdin positional.
    pub(crate) fn invocation(&self, path: Option<&str>) -> String {
        format!(
            "prose format --select {} {}",
            self.slugs(),
            path.unwrap_or("-"),
        )
    }

    /// The reproducing subset as one comma-separated `--select` value.
    pub(crate) fn slugs(&self) -> String {
        self.rules.iter().map(RuleId::as_str).join(",")
    }
}

#[cfg(test)]
impl UnstableRewrite {
    /// A report over a widening rewrite under `slug`, the shape the
    /// renderer and notice tests measure against.
    pub(crate) fn sample(slug: &'static str) -> Self {
        Self {
            config_toml: String::new(),
            first: "yy = 1\n".to_owned(),
            rules: vec![RuleId::from(slug)],
            second: "yyy = 1\n".to_owned(),
        }
    }
}

/// The clause placing the defect in Prose rather than in the source,
/// singular for one file and plural for a folded group.
pub(crate) fn blame(files: usize) -> String {
    let subject = if files == 1 { "the file" } else { "the files" };
    format!("The defect lies in prose rather than in {subject}")
}

/// The sentence naming `subject`'s rewrite as one a second run would
/// change, shared by the terminal notice and the editor message.
pub(crate) fn headline(subject: &str) -> String {
    format!("prose rewrote {subject} to output a second run would change")
}

/// The rules editing `original` together with the `editing` rules still
/// editing the output, the `editing` rules leading so the pairs the
/// probe budget reaches first are the ones anchored on a rule the
/// settle walk already named. Each block keeps registration order.
fn candidates(pipeline: &Pipeline, original: &str, editing: &[RuleId]) -> Vec<RuleId> {
    let touched = original
        .parse::<Source>()
        .map(|source| pipeline.unsettled(&source))
        .unwrap_or_default();
    pipeline
        .rule_ids()
        .filter(|rule| touched.contains(rule) || editing.contains(rule))
        .sorted_by_key(|rule| !editing.contains(rule))
        .collect()
}

/// The pipeline seating `rules` alone under `config`.
fn filtered(config: &Config) -> impl Fn(&[RuleId]) -> Pipeline + '_ {
    move |rules| Pipeline::with_filters(config, rules, &[])
}

/// `None` where neither a rule alone nor a rule pair reproduces on this
/// source.
fn narrowed(
    pipeline: &Pipeline,
    config: &Config,
    original: &str,
    editing: &[RuleId],
) -> Option<Vec<RuleId>> {
    reproducing_subset(
        &candidates(pipeline, original, editing),
        original,
        filtered(config),
    )
}

/// Falls back to `formatted`'s own text where the second run does not
/// parse or a rule's output is rejected. The second source carries the
/// cell boundaries the first one did.
fn second_pass(pipeline: &Pipeline, formatted: &Source) -> String {
    let first = formatted.text();
    formatted
        .reparse_carrying(first.to_owned(), formatted.cell_offsets().clone())
        .ok()
        .and_then(|source| pipeline.run(source).ok())
        .map_or_else(
            || first.to_owned(),
            |(settled, _)| settled.text().to_owned(),
        )
}

/// The reproducing subset for a report over `formatted`, taking the
/// whole selection for a notebook and wherever `narrow` finds none.
fn subset(
    pipeline: &Pipeline,
    formatted: &Source,
    narrow: impl FnOnce() -> Option<Vec<RuleId>>,
) -> Vec<RuleId> {
    (!formatted.is_notebook())
        .then(narrow)
        .flatten()
        .unwrap_or_else(|| pipeline.rule_ids().collect())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::{
        diagnostics::fired_rules,
        testing::{breaks_parse, never_settles, notebook, parse, prefix_rule},
    };

    /// What `widening()` writes over `SOURCE`, standing in for the
    /// bytes a prior write-back run marked as its own output.
    const MARKED: &str = "yy = 1\n";
    const SOURCE: &str = "x = 1\n";

    /// A pipeline whose first rule settles after rewriting `x` to `q`,
    /// whose second edits only a `q`-leading buffer and never settles
    /// on its own output, and whose third never fires.
    fn downstream() -> Pipeline {
        Pipeline::from_rules(vec![
            Box::new(prefix_rule("settles-once", "x", "q")),
            Box::new(prefix_rule("widens-downstream", "q", "qq")),
            Box::new(prefix_rule("never-fires", "zzz", "z")),
        ])
    }

    fn widening() -> Pipeline {
        Pipeline::from_rules(vec![Box::new(never_settles("widener"))])
    }

    #[test]
    fn detect_narrowed_leaves_a_rule_silent_on_the_first_pass_to_the_full_walk() {
        let pipeline = Pipeline::from_rules(vec![
            Box::new(prefix_rule("edits-q", "q", "qq")),
            Box::new(prefix_rule("settles-once", "x", "q")),
        ]);
        let config = Config::default();
        let (formatted, diagnostics) = pipeline.run(parse(SOURCE)).expect("runs");

        assert!(UnstableRewrite::detect(&pipeline, &config, SOURCE, &formatted).is_some());
        assert!(
            UnstableRewrite::detect_narrowed(
                &pipeline,
                &config,
                SOURCE,
                &formatted,
                &fired_rules(&diagnostics),
            )
            .is_none(),
            "`edits-q` emitted nothing on the first pass, so the narrowed pass leaves it to the full walk",
        );
    }

    #[test]
    fn detect_narrowed_reports_a_rule_that_fired_and_still_edits() {
        let pipeline = downstream();
        let config = Config::default();
        let (formatted, diagnostics) = pipeline.run(parse(SOURCE)).expect("runs");

        let full = UnstableRewrite::detect(&pipeline, &config, SOURCE, &formatted)
            .expect("`widens-downstream` leaves the output unsettled");
        let narrowed = UnstableRewrite::detect_narrowed(
            &pipeline,
            &config,
            SOURCE,
            &formatted,
            &fired_rules(&diagnostics),
        )
        .expect("`widens-downstream` fired on the first pass and still edits");

        assert_eq!(narrowed, full);
    }

    #[test]
    fn candidates_reach_a_rule_that_edits_only_the_output_and_lead_with_it() {
        let pipeline = downstream();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");
        let editing = pipeline.unsettled(&formatted);

        assert_eq!(
            candidates(&pipeline, SOURCE, &editing),
            vec![
                RuleId::from("widens-downstream"),
                RuleId::from("settles-once"),
            ],
            "the rule still editing the output leads, so its pairs sit inside the probe budget",
        );
    }

    #[test]
    fn detect_carries_both_passes_of_a_widening_rewrite() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");

        let report = UnstableRewrite::detect(&pipeline, &Config::default(), SOURCE, &formatted)
            .expect("a widening rule leaves the output unsettled");

        assert_eq!(report.first, "yy = 1\n");
        assert_eq!(report.second, "yyy = 1\n");
        assert!(report.config_toml.is_empty(), "{}", report.config_toml);
    }

    #[test]
    fn detect_carries_the_keys_the_config_set_away_from_the_default() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");
        let config = Config {
            code_line_length: NonZeroUsize::new(100),
            ..Config::default()
        };

        let report = UnstableRewrite::detect(&pipeline, &config, SOURCE, &formatted)
            .expect("a widening rule leaves the output unsettled");

        assert_eq!(report.config_toml, "code-line-length = 100\n");
    }

    #[test]
    fn detect_holds_the_first_pass_where_a_second_run_is_rejected() {
        // `breaks-parse` still edits the buffer it is handed, so the
        // report opens, and its own output is what the second run
        // rejects.
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let formatted = parse(SOURCE);

        let report = UnstableRewrite::detect(&pipeline, &Config::default(), SOURCE, &formatted)
            .expect("the rule still edits the buffer");

        assert_eq!(report.first, SOURCE);
        assert_eq!(report.second, report.first);
    }

    #[test]
    fn detect_is_none_for_a_settled_rewrite() {
        let pipeline = Pipeline::with_defaults(&Config::default());
        let (formatted, _) = pipeline.run(parse("alpha = 1\nb = 22\n")).expect("runs");

        assert!(
            UnstableRewrite::detect(
                &pipeline,
                &Config::default(),
                "alpha = 1\nb = 22\n",
                &formatted
            )
            .is_none()
        );
    }

    #[test]
    fn detect_marked_is_none_where_the_marked_bytes_do_not_parse() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");

        assert!(
            UnstableRewrite::detect_marked(&pipeline, &Config::default(), "def (\n", &formatted)
                .is_none()
        );
    }

    #[test]
    fn detect_marked_is_none_where_the_marked_bytes_settle() {
        let pipeline = Pipeline::with_defaults(&Config::default());
        let (formatted, _) = pipeline.run(parse("alpha = 1\nb = 22\n")).expect("runs");

        assert!(
            UnstableRewrite::detect_marked(
                &pipeline,
                &Config::default(),
                formatted.text(),
                &formatted
            )
            .is_none()
        );
    }

    #[test]
    fn detect_marked_names_the_whole_selection_for_a_notebook() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(notebook(&[MARKED])).expect("runs");

        let report =
            UnstableRewrite::detect_marked(&pipeline, &Config::default(), MARKED, &formatted)
                .expect("the marked bytes are still unsettled");

        assert_eq!(report.rules, pipeline.rule_ids().collect::<Vec<_>>());
    }

    #[test]
    fn detect_marked_reads_the_editing_set_off_the_marked_bytes() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(MARKED)).expect("runs");

        let report =
            UnstableRewrite::detect_marked(&pipeline, &Config::default(), MARKED, &formatted)
                .expect("the marked bytes are still unsettled");

        assert_eq!(report.rules, vec![RuleId::from("widener")]);
        assert_eq!(report.first, "yyy = 1\n");
        assert_eq!(report.second, "yyyy = 1\n");
    }

    #[test]
    fn detect_names_the_whole_selection_for_a_notebook() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(notebook(&[SOURCE])).expect("runs");

        let report = UnstableRewrite::detect(&pipeline, &Config::default(), SOURCE, &formatted)
            .expect("a widening rule leaves the output unsettled");

        assert_eq!(report.rules, pipeline.rule_ids().collect::<Vec<_>>());
    }

    #[test]
    fn detect_reports_regardless_of_the_config_key() {
        // The key gates the notice surfaces, not the detector, so a
        // caller that wants the check despite the key, as `check
        // --validate` does, reads a real answer here.
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");
        let config = Config {
            report_unstable_output: false,
            ..Config::default()
        };

        assert!(UnstableRewrite::detect(&pipeline, &config, SOURCE, &formatted).is_some());
    }

    #[test]
    fn detect_runs_a_notebooks_second_pass_across_its_cells() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(notebook(&[SOURCE, "y = 2\n"])).expect("runs");

        let report = UnstableRewrite::detect(&pipeline, &Config::default(), SOURCE, &formatted)
            .expect("a widening rule leaves the output unsettled");

        assert!(report.second.contains("y = 2"), "{}", report.second);
        assert_ne!(report.second, report.first);
    }

    #[test]
    fn invocation_names_the_reproducing_subset_and_the_path() {
        let mut report = UnstableRewrite::sample("align-equals");
        report.rules.push(RuleId::from("align-colons"));

        assert_eq!(
            report.invocation(Some("src/module.py")),
            "prose format --select align-equals,align-colons src/module.py",
        );
    }

    #[test]
    fn invocation_names_the_stdin_positional_without_a_path() {
        assert_eq!(
            UnstableRewrite::sample("align-equals").invocation(None),
            "prose format --select align-equals -",
        );
    }
}

//! The report over a rewrite whose settle check names rules.
//!
//! A run that rewrote a file re-applies its enabled rules to the output
//! it produced. Where any still edits, the rewrite is a defect in Prose
//! rather than in the file beneath it, and this module builds the record
//! the CLI and the language server both render: the narrowed rule
//! subset that reproduces it, the output the run wrote, and what a
//! second pass turns that output into.

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{config::Config, pipeline::Pipeline, rule::RuleId, source::Source};

mod form;
mod narrow;

pub(crate) use form::report_url;
use narrow::reproducing_subset;

/// One file's rewrite that a second pass would change.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct UnstableRewrite {
    /// The `[tool.prose]` table that governed the run.
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
    /// The report over `formatted`, the output `pipeline` produced for
    /// `original`, or `None` where `config` turns the report off or the
    /// enabled rules leave the output settled. The subset narrows to a
    /// rule alone or a rule pair where one reproduces, falling back to
    /// the whole selection the run carried, which a notebook takes
    /// outright.
    pub(crate) fn detect(
        pipeline: &Pipeline,
        config: &Config,
        original: &str,
        formatted: &Source,
    ) -> Option<Self> {
        if !config.report_unstable_output || pipeline.unsettled(formatted).is_empty() {
            return None;
        }
        let first = formatted.text().to_owned();
        let rules = (!formatted.is_notebook())
            .then(|| narrowed(pipeline, config, original))
            .flatten()
            .unwrap_or_else(|| pipeline.rule_ids().collect());
        let second = second_pass(pipeline, formatted);
        Some(Self {
            config_toml: config.to_toml(),
            first,
            rules,
            second,
        })
    }

    /// The `prose format` invocation reproducing the defect against
    /// `path`, which confirms it gone once a later release fixes it. A
    /// subject naming no path takes the `-` stdin positional.
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

/// The reproducing subset for `original`, searched over the rules that
/// edit it. `None` where neither a rule alone nor a rule pair
/// reproduces on this source.
fn narrowed(pipeline: &Pipeline, config: &Config, original: &str) -> Option<Vec<RuleId>> {
    let candidates = original
        .parse::<Source>()
        .map(|source| pipeline.unsettled(&source))
        .unwrap_or_default();
    reproducing_subset(&candidates, original, |subset| {
        Pipeline::with_filters(config, subset, &[])
    })
}

/// What running `pipeline` over `formatted` a second time produces,
/// `formatted`'s own text where that run does not parse or a rule's
/// output is rejected. The second source carries the cell boundaries
/// the first one did.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{breaks_parse, never_settles, notebook, parse};

    const SOURCE: &str = "x = 1\n";

    fn widening() -> Pipeline {
        Pipeline::from_rules(vec![Box::new(never_settles("widener"))])
    }

    #[test]
    fn detect_carries_both_passes_of_a_widening_rewrite() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");

        let report = UnstableRewrite::detect(&pipeline, &Config::default(), SOURCE, &formatted)
            .expect("a widening rule leaves the output unsettled");

        assert_eq!(report.first, "yy = 1\n");
        assert_eq!(report.second, "yyy = 1\n");
        assert!(report.config_toml.contains("code-line-length = 88"));
    }

    #[test]
    fn detect_holds_quiet_where_the_config_turns_the_report_off() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(parse(SOURCE)).expect("runs");
        let config = Config {
            report_unstable_output: false,
            ..Config::default()
        };

        assert!(UnstableRewrite::detect(&pipeline, &config, SOURCE, &formatted).is_none());
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
    fn detect_names_the_whole_selection_for_a_notebook() {
        let pipeline = widening();
        let (formatted, _) = pipeline.run(notebook(&[SOURCE])).expect("runs");

        let report = UnstableRewrite::detect(&pipeline, &Config::default(), SOURCE, &formatted)
            .expect("a widening rule leaves the output unsettled");

        assert_eq!(report.rules, pipeline.rule_ids().collect::<Vec<_>>());
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

//! Outcome aggregation: summaries, exit-status derivation, and
//! diagnostic emission.

use std::io::{self, Write};

use anyhow::Context;

use super::{FileOutcome, Mode, has_format_change};
use crate::{
    cache::Rewrite,
    cli::{
        args::OutputFormat,
        emit::{Emitter, EmitterSummary, Github, Json, Run, Sarif, Text},
        exit_status::ExitStatus,
        output::{self, Presentation, Summary},
    },
    diagnostics::Diagnostic,
};

pub(super) fn emit_outcomes<W: Write>(
    outcomes: &[FileOutcome],
    format: OutputFormat,
    writer: &mut W,
    summary: &EmitterSummary,
) -> anyhow::Result<()> {
    let view: Vec<Run<'_>> = outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Done {
                diagnostics,
                file,
                notebook_index,
                ..
            } => Some(Run::new(
                file,
                diagnostics.as_slice(),
                notebook_index.as_deref(),
            )),
            FileOutcome::Failed(_) => None,
        })
        .collect();
    match format {
        OutputFormat::Github => Github.emit(writer, &view, summary),
        OutputFormat::Json => Json.emit(writer, &view, summary),
        OutputFormat::Sarif => Sarif.emit(writer, &view, summary),
        OutputFormat::Text => Text::new().emit(writer, &view, summary),
    }?;
    writer.flush().context("flushing stdout")?;
    Ok(())
}

pub(super) fn emitter_summary(outcomes: &[FileOutcome]) -> EmitterSummary {
    outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Done {
                diagnostics,
                rewrite,
                ..
            } => Some((diagnostics, rewrite)),
            FileOutcome::Failed(_) => None,
        })
        .fold(
            EmitterSummary::default(),
            |mut summary, (diagnostics, rewrite)| {
                summary.files_visited += 1;
                summary.files_changed += usize::from(file_changed(diagnostics, rewrite));
                summary.files_with_diagnostics += usize::from(!diagnostics.is_empty());
                summary.diagnostics_total += diagnostics.len();
                for diag in diagnostics {
                    summary.lint_total += usize::from(diag.severity.is_lint());
                    *summary.rules_fired.entry(diag.rule).or_default() += 1;
                }
                summary
            },
        )
}

/// A file counts as changed when `run` produced text differing from the
/// original. A mode that skipped the rewrite falls back to whether
/// `diagnose` emitted a format diagnostic.
pub(super) fn file_changed(diagnostics: &[Diagnostic], rewrite: &Rewrite) -> bool {
    match rewrite {
        Rewrite::Changed(_) => true,
        Rewrite::Skipped => has_format_change(diagnostics),
        Rewrite::Unchanged => false,
    }
}

pub(super) fn finish(
    outcomes: &[FileOutcome],
    cache_enabled: bool,
    verbose: bool,
    demote_format_change: bool,
) -> ExitStatus {
    if verbose {
        report_verbose(outcomes, cache_enabled, &mut io::stderr());
    }
    status_from_outcomes(outcomes, demote_format_change)
}

/// The surviving-lint disclosure a text-format `format` run appends
/// after its outcome line, `None` for a check run, a structured output
/// whose emitters already printed the lint, or a run leaving none.
fn lint_remainder(summary: &EmitterSummary, mode: Mode, text_output: bool) -> Option<Summary> {
    let total = summary.lint_total;
    let discloses = !matches!(mode, Mode::Check) && text_output && total > 0;
    discloses.then_some(Summary::LintRemainder { total })
}

/// Writes a run's closing summary: the rewrite or diagnostics outcome,
/// then in a format mode whose diagnostics never reached stdout the
/// surviving-lint disclosure. `text_output` is true for a text-format
/// `format` run, whose lint the structured emitters never printed.
pub(super) fn render_summary<E: Write>(
    stderr: &mut E,
    present: &Presentation,
    outcomes: &[FileOutcome],
    summary: &EmitterSummary,
    mode: Mode,
    text_output: bool,
) {
    let lines = summarize(outcomes, summary, mode)
        .into_iter()
        .chain(lint_remainder(summary, mode, text_output));
    for line in lines {
        let _ = output::report(stderr, present, &line);
    }
}

pub(super) fn report_verbose<W: Write>(
    outcomes: &[FileOutcome],
    cache_enabled: bool,
    writer: &mut W,
) {
    if !cache_enabled {
        let _ = writeln!(writer, "cache: bypassed");
        return;
    }
    let (hits, misses) = outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Done { cached, .. } => Some(*cached),
            FileOutcome::Failed(_) => None,
        })
        .fold(
            (0_usize, 0_usize),
            |(h, m), c| if c { (h + 1, m) } else { (h, m + 1) },
        );
    let _ = writeln!(
        writer,
        "cache: {hits} hits, {misses} misses, {total} files",
        total = hits + misses,
    );
}

pub(super) fn status_from_outcomes(
    outcomes: &[FileOutcome],
    demote_format_change: bool,
) -> ExitStatus {
    outcomes
        .iter()
        .map(|outcome| match outcome {
            FileOutcome::Done {
                diagnostics,
                rewrite,
                ..
            } => {
                // A rewrite that settled back to the input byte-for-byte
                // reports clean, its cancelling edits notwithstanding.
                let demote = demote_format_change || matches!(rewrite, Rewrite::Unchanged);
                diagnostics
                    .iter()
                    .map(|d| ExitStatus::from(d.severity))
                    .filter(|s| !demote || *s != ExitStatus::FormatChange)
                    .max()
                    .unwrap_or_default()
            }
            FileOutcome::Failed(s) => *s,
        })
        .max()
        .unwrap_or_default()
}

/// Resolves an outcome set into its closing [`Summary`], or `None` when
/// the clean line is suppressed, either by a per-file failure already
/// logged to stderr or by a format run leaving lint whose disclosure is
/// the lint-remainder line instead.
fn summarize(outcomes: &[FileOutcome], summary: &EmitterSummary, mode: Mode) -> Option<Summary> {
    let failed = outcomes.iter().any(|o| matches!(o, FileOutcome::Failed(_)));
    let resolved = match mode {
        Mode::Check => match summary.diagnostics_total {
            0 => Summary::Clean,
            total => Summary::Diagnostics {
                files: summary.files_with_diagnostics,
                total,
            },
        },
        Mode::Preview => match summary.files_changed {
            0 => Summary::Clean,
            files => Summary::WouldReformat { files },
        },
        Mode::Reformat => match summary.files_changed {
            0 => Summary::Clean,
            files => Summary::Reformatted { files },
        },
    };
    match resolved {
        Summary::Clean if failed || summary.lint_total > 0 => None,
        resolved => Some(resolved),
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;
    use ruff_diagnostics::{Edit, Fix};
    use ruff_text_size::TextRange;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::RuleId;
    use crate::source::Source;
    use crate::testing::{FailingWriter, parse, range};

    fn diagnostic(severity: Severity, range: TextRange, slug: &'static str) -> Diagnostic {
        Diagnostic {
            fix: severity
                .is_format()
                .then(|| Fix::safe_edit(Edit::range_replacement("y".into(), range))),
            message: "test".into(),
            range,
            rule: RuleId::from(slug),
            severity,
        }
    }

    fn outcome_with(source: Source, diagnostics: Vec<Diagnostic>) -> FileOutcome {
        FileOutcome::Done {
            cached: false,
            diagnostics,
            file: source.source_file().clone(),
            notebook_index: None,
            rewrite: Rewrite::Skipped,
        }
    }

    #[test]
    fn check_outcomes_with_failed_parse_takes_higher_status() {
        let source = parse("x = 1\n");
        let range = range(0, 1);
        let outcomes = vec![
            outcome_with(
                source,
                vec![diagnostic(Severity::Format, range, "synthetic-format")],
            ),
            FileOutcome::Failed(ExitStatus::ParseError),
        ];

        let status = status_from_outcomes(&outcomes, false);

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn check_outcomes_with_lint_and_format_returns_lint_violation() {
        let source = parse("x = 1\n");
        let range = range(0, 1);
        let diagnostics = vec![
            diagnostic(Severity::Format, range, "synthetic-format"),
            diagnostic(Severity::Lint, range, "synthetic-lint"),
        ];
        let outcomes = vec![outcome_with(source, diagnostics)];

        let status = status_from_outcomes(&outcomes, false);

        assert_eq!(status, ExitStatus::LintViolation);
    }

    #[test]
    fn check_outcomes_with_synthetic_lint_returns_lint_violation() {
        let source = parse("x = 1\n");
        let diagnostics = vec![diagnostic(Severity::Lint, range(0, 1), "synthetic-lint")];
        let outcomes = vec![outcome_with(source, diagnostics)];

        let status = status_from_outcomes(&outcomes, false);

        assert_eq!(status, ExitStatus::LintViolation);
    }

    #[test]
    fn emit_outcomes_propagates_the_writer_failure_kind() {
        let source = parse("x = 1\n");
        let diags = vec![diagnostic(
            Severity::Format,
            range(0, 1),
            "synthetic-format",
        )];
        let outcomes = vec![outcome_with(source, diags)];
        let result = emit_outcomes(
            &outcomes,
            OutputFormat::Json,
            &mut FailingWriter(io::ErrorKind::BrokenPipe),
            &EmitterSummary::default(),
        );
        let err = result.expect_err("writer failure propagates");
        assert_matches!(
            err.downcast_ref::<io::Error>(),
            Some(e) if e.kind() == io::ErrorKind::BrokenPipe
        );
    }

    #[rstest]
    fn emit_outcomes_renders_each_output_format(
        #[values(
            OutputFormat::Github,
            OutputFormat::Json,
            OutputFormat::Sarif,
            OutputFormat::Text
        )]
        format: OutputFormat,
    ) {
        let source = parse("x = 1\n");
        let outcomes = vec![outcome_with(source, Vec::new())];
        let mut buf = Vec::new();
        emit_outcomes(&outcomes, format, &mut buf, &EmitterSummary::default()).expect("emits");
    }

    #[test]
    fn emitter_summary_counts_visited_changed_diagnostics_and_rules() {
        let range = range(0, 1);
        let mut changed = outcome_with(
            parse("x = 1\n"),
            vec![
                diagnostic(Severity::Format, range, "align-equals"),
                diagnostic(Severity::Lint, range, "reassigned-constants"),
            ],
        );
        if let FileOutcome::Done { rewrite, .. } = &mut changed {
            *rewrite = Rewrite::text("x   = 1\n".to_owned());
        }
        let clean = outcome_with(parse("y = 2\n"), Vec::new());
        let outcomes = vec![changed, clean, FileOutcome::Failed(ExitStatus::ParseError)];

        let summary = emitter_summary(&outcomes);

        assert_eq!(summary.files_visited, 2);
        assert_eq!(summary.files_changed, 1);
        assert_eq!(summary.files_with_diagnostics, 1);
        assert_eq!(summary.diagnostics_total, 2);
        assert_eq!(summary.lint_total, 1);
        assert_eq!(summary.rules_fired[&RuleId::from("align-equals")], 1);
        assert_eq!(
            summary.rules_fired[&RuleId::from("reassigned-constants")],
            1
        );
    }

    #[test]
    fn emitter_summary_tallies_repeated_rule_occurrences() {
        let range = range(0, 1);
        let outcome = outcome_with(
            parse("x = 1\n"),
            vec![
                diagnostic(Severity::Format, range, "align-equals"),
                diagnostic(Severity::Format, range, "align-equals"),
            ],
        );

        let summary = emitter_summary(std::slice::from_ref(&outcome));

        assert_eq!(summary.rules_fired[&RuleId::from("align-equals")], 2);
    }

    #[test]
    fn file_changed_counts_a_changed_rewrite_or_a_skipped_format_diagnostic() {
        let range = range(0, 1);
        let format = vec![diagnostic(Severity::Format, range, "synthetic-format")];
        let lint = vec![diagnostic(Severity::Lint, range, "synthetic-lint")];

        assert!(file_changed(&[], &Rewrite::text("x = 1\n".to_owned())));
        assert!(file_changed(&format, &Rewrite::Skipped));
        assert!(!file_changed(&format, &Rewrite::Unchanged));
        assert!(!file_changed(&lint, &Rewrite::Skipped));
        assert!(!file_changed(&[], &Rewrite::Skipped));
    }

    #[rstest]
    #[case(Mode::Check, true, 2, None)]
    #[case(Mode::Reformat, true, 0, None)]
    #[case(Mode::Reformat, false, 2, None)]
    #[case(Mode::Preview, true, 3, Some(3))]
    #[case(Mode::Reformat, true, 1, Some(1))]
    fn lint_remainder_discloses_only_text_format_lint(
        #[case] mode: Mode,
        #[case] text_output: bool,
        #[case] lint_total: usize,
        #[case] expected: Option<usize>,
    ) {
        let summary = EmitterSummary {
            lint_total,
            ..EmitterSummary::default()
        };
        let got = lint_remainder(&summary, mode, text_output);
        match expected {
            None => assert_matches!(got, None),
            Some(total) => {
                assert_matches!(got, Some(Summary::LintRemainder { total: t }) if t == total)
            }
        }
    }

    #[test]
    fn report_verbose_prints_bypassed_when_cache_disabled() {
        let mut buf = Vec::new();
        report_verbose(&[], false, &mut buf);
        assert_eq!(String::from_utf8(buf).expect("utf-8"), "cache: bypassed\n");
    }

    #[test]
    fn report_verbose_prints_hit_and_miss_counts() {
        let make = |cached: bool| {
            let source = parse("x = 1\n");
            let mut o = outcome_with(source, Vec::new());
            if let FileOutcome::Done { cached: c, .. } = &mut o {
                *c = cached;
            }
            o
        };
        let outcomes = vec![
            make(true),
            make(true),
            make(false),
            FileOutcome::Failed(ExitStatus::Clean),
        ];
        let mut buf = Vec::new();
        report_verbose(&outcomes, true, &mut buf);
        assert_eq!(
            String::from_utf8(buf).expect("utf-8"),
            "cache: 2 hits, 1 misses, 3 files\n",
        );
    }

    #[test]
    fn status_from_outcomes_clears_format_change_for_an_unchanged_rewrite() {
        let source = parse("x = 1\n");
        let range = range(0, 1);
        let mut outcome = outcome_with(
            source,
            vec![
                diagnostic(Severity::Format, range, "synthetic-format"),
                diagnostic(Severity::Lint, range, "synthetic-lint"),
            ],
        );
        if let FileOutcome::Done { rewrite, .. } = &mut outcome {
            *rewrite = Rewrite::Unchanged;
        }
        let outcomes = vec![outcome];

        assert_eq!(
            status_from_outcomes(&outcomes, false),
            ExitStatus::LintViolation,
        );
    }

    #[test]
    fn status_from_outcomes_demotes_format_change_when_demoted() {
        let source = parse("x = 1\n");
        let outcomes = vec![outcome_with(
            source,
            vec![diagnostic(
                Severity::Format,
                range(0, 1),
                "synthetic-format",
            )],
        )];
        assert_eq!(status_from_outcomes(&outcomes, true), ExitStatus::Clean);
        assert_eq!(
            status_from_outcomes(&outcomes, false),
            ExitStatus::FormatChange,
        );
    }

    #[test]
    fn summarize_reports_diagnostics_alongside_a_failure() {
        let source = parse("x = 1\n");
        let outcomes = vec![
            outcome_with(
                source,
                vec![diagnostic(Severity::Lint, range(0, 1), "synthetic-lint")],
            ),
            FileOutcome::Failed(ExitStatus::ParseError),
        ];
        assert_matches!(
            summarize(&outcomes, &emitter_summary(&outcomes), Mode::Check),
            Some(Summary::Diagnostics { files: 1, total: 1 })
        );
    }

    #[test]
    fn summarize_reports_reformatted_alongside_surviving_lint() {
        let summary = EmitterSummary {
            files_changed: 2,
            lint_total: 1,
            ..EmitterSummary::default()
        };
        assert_matches!(
            summarize(&[], &summary, Mode::Reformat),
            Some(Summary::Reformatted { files: 2 })
        );
    }

    #[test]
    fn summarize_suppresses_clean_summary_when_a_file_failed() {
        let outcomes = vec![FileOutcome::Failed(ExitStatus::ParseError)];
        assert!(summarize(&outcomes, &emitter_summary(&outcomes), Mode::Check).is_none());
    }

    #[test]
    fn summarize_suppresses_the_clean_line_when_lint_survives_a_format_run() {
        let summary = EmitterSummary {
            lint_total: 1,
            ..EmitterSummary::default()
        };
        assert!(summarize(&[], &summary, Mode::Reformat).is_none());
    }
}

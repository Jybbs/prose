//! Outcome aggregation: summaries, exit-status derivation, and
//! diagnostic emission.

use std::io::{self, Write};

use anyhow::Context;

use super::{FileOutcome, Mode, RunSetup, has_format_change};
use crate::{
    cache::Rewrite,
    cli::{
        args::OutputFormat,
        exit_status::ExitStatus,
        output::{self, Presentation, Summary},
    },
    diagnostics::{Diagnostic, Emitter, EmitterSummary, Github, Json, Run, Sarif, Text},
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
                file, diagnostics, ..
            } => Some((file, diagnostics.as_slice())),
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
    setup: &RunSetup,
    verbose: bool,
    demote_format_change: bool,
) -> ExitStatus {
    if verbose {
        report_verbose(outcomes, setup.cache.is_some(), &mut io::stderr());
    }
    status_from_outcomes(outcomes, demote_format_change)
}

pub(super) fn render_summary<E: Write>(
    stderr: &mut E,
    present: &Presentation,
    summary: Option<Summary>,
) {
    if let Some(summary) = summary {
        let _ = output::report(stderr, present, &summary);
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

/// Resolves an outcome set into its closing [`Summary`], or `None`
/// when a clean run is shadowed by a per-file failure already logged
/// to stderr.
pub(super) fn summarize(
    outcomes: &[FileOutcome],
    summary: &EmitterSummary,
    mode: Mode,
) -> Option<Summary> {
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
        Summary::Clean if failed => None,
        resolved => Some(resolved),
    }
}

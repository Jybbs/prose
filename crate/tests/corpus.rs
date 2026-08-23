//! Corpus sweep at every configured line length: the text the formatter
//! writes leaves no rule rewriting it and no reported fix unapplied.
//!
//! The two defects share a shape and not a mechanism. A rewrite a second
//! pass would change leaves a rule whose edits still splice, which
//! [`Pipeline::unsettled`] reports. A fix the weave drops leaves a rule
//! whose diagnostic still names an edit that never lands, which only the
//! diagnostic pass reports, since the dropped edit splices back to the
//! text it started from. The runtime settle check reads the first alone
//! and reads it only over a file the pass rewrote, so this sweep reads
//! both over every file. A run that panics is recorded against its file
//! rather than ending the sweep, so one defect leaves the rest visible.
//! `PROSE_SETTLE_CORPUS` points the sweep at a directory other than the
//! fixture tree.

use std::{
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    path::Path,
};

use itertools::Itertools;
use prose::{
    config::Config, diagnostics::Severity, pipeline::Pipeline, rule::render_slugs, source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use common::{Tally, corpus};

mod common;

/// The line lengths the sweep covers, the default flanked by the narrow
/// and wide settings a project sets it to.
const WIDTHS: [usize; 6] = [40, 50, 60, 79, 88, 100];

/// The defects one width's pass over the corpus found.
#[derive(Default)]
struct Findings {
    /// Runs that panicked, keyed by the message.
    panicked: Tally,
    /// Outputs carrying a reported fix the run never applied.
    unapplied: Tally,
    /// Outputs a rule still rewrites.
    unsettled: Tally,
}

impl Findings {
    fn absorb(&mut self, other: Self) {
        self.panicked.absorb(other.panicked);
        self.unapplied.absorb(other.unapplied);
        self.unsettled.absorb(other.unsettled);
    }

    fn total(&self) -> usize {
        self.panicked.len() + self.unapplied.len() + self.unsettled.len()
    }
}

/// The default configuration at `width`.
fn config_at(width: usize) -> Config {
    Config {
        code_line_length: NonZeroUsize::new(width),
        ..Config::default()
    }
}

/// The message a caught panic carried, `"panicked"` for a payload of
/// neither string type.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "panicked".to_owned())
}

/// Formats `path` under `pipeline` and records what the output leaves
/// behind, the run wrapped so a panic files against the file it read.
fn probe(pipeline: &Pipeline, width: usize, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        return findings;
    };
    let ran = panic::catch_unwind(AssertUnwindSafe(|| {
        let (formatted, _) = pipeline.run(source).ok()?;
        let editing = pipeline.unsettled(&formatted);
        let unlanded: Vec<_> = pipeline
            .diagnose(&formatted)
            .into_iter()
            .filter(|d| d.severity == Severity::Format && d.fix.is_some())
            .map(|d| d.rule)
            .filter(|rule| !editing.contains(rule))
            .unique()
            .collect();
        Some((editing, unlanded))
    }));
    let outcome = match ran {
        Ok(outcome) => outcome,
        Err(payload) => {
            findings.panicked.record(
                format!("at {width}, the run panicked: {}", panic_message(&*payload)),
                path,
            );
            return findings;
        }
    };
    let Some((editing, unlanded)) = outcome else {
        return findings;
    };
    if !editing.is_empty() {
        findings.unsettled.record(
            format!("at {width}, {} rewrites the output", render_slugs(&editing)),
            path,
        );
    }
    if !unlanded.is_empty() {
        findings.unapplied.record(
            format!(
                "at {width}, {} reports a fix the output never took",
                render_slugs(&unlanded)
            ),
            path,
        );
    }
    findings
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_width_settles_and_applies_what_it_reports() {
    let files = corpus();
    assert!(!files.is_empty(), "the corpus holds no `.py` files");
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let mut findings = Findings::default();
    for width in WIDTHS {
        let pipeline = Pipeline::with_defaults(&config_at(width));
        findings.absorb(
            files
                .par_iter()
                .map(|path| probe(&pipeline, width, path))
                .reduce(Findings::default, |mut held, next| {
                    held.absorb(next);
                    held
                }),
        );
    }
    panic::set_hook(previous);
    let report = format!(
        "{}{}{}",
        findings.panicked.render("runs that panicked"),
        findings
            .unsettled
            .render("rewrites a second pass would change"),
        findings.unapplied.render("fixes the output never took"),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across {} files at {} widths:{report}",
        findings.total(),
        files.len(),
        WIDTHS.len(),
    );
}

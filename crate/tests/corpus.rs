//! Corpus sweep at every configured line length: the text the formatter
//! writes leaves no rule rewriting it and no reported fix unapplied.
//! [`Pipeline::settle_report`] reads both defects off one walk over
//! every file's output. A run that panics or is rejected is recorded
//! against its file rather than ending the sweep, and a file passing
//! `BUDGET` stops the sweep and names itself. Each width in [`WIDTHS`]
//! runs once per axis in [`Axis::ALL`], one budget varied and the rest at
//! their defaults.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    path::Path,
};

use itertools::Itertools;
use prose::{
    config::Config,
    diagnostics::Severity,
    pipeline::{Pipeline, PipelineError, SettleReport},
    rule::{RuleId, render_slugs},
    source::Source,
};

use common::{
    Absorbing, CORPUS, Hit, Slot, Tally, WIDTHS, WIDTHS_VAR, corpus, env_list, excerpt,
    note_verified, report_verified, swept, unread, verifying, widths_or,
};

mod common;

/// The environment variable narrowing the axes by name.
const AXES_VAR: &str = "PROSE_SETTLE_AXES";

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

thread_local! {
    /// The defect line the silent hook last rendered for a panic, read
    /// back by the probe that caught it.
    static PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The budget an axis varies at each width, every other budget held at
/// its default.
#[derive(Clone, Copy)]
enum Axis {
    /// `code_line_length` varied.
    Code,
    /// `docstring_line_length` varied.
    Docstring,
    /// `code_line_length` varied with `import_line_length` unset, so
    /// the import budget falls back to the varied code budget.
    Fallback,
    /// `import_line_length` varied.
    Import,
}

impl Axis {
    /// Every axis the sweep crosses with each width absent [`AXES_VAR`],
    /// each varying the budget it names.
    const ALL: [Self; 4] = [Self::Code, Self::Docstring, Self::Fallback, Self::Import];

    /// The phrase a finding and a `Slot` label name this axis and
    /// width by.
    fn clause(self, width: usize) -> String {
        format!("{} {width}", self.label())
    }

    /// The default configuration with this axis's budget at `width`.
    fn config(self, width: usize) -> Config {
        let budget = NonZeroUsize::new(width);
        match self {
            Self::Code => Config {
                code_line_length: budget,
                ..Config::default()
            },
            Self::Docstring => Config {
                docstring_line_length: budget,
                ..Config::default()
            },
            Self::Fallback => Config {
                code_line_length: budget,
                import_line_length: None,
                ..Config::default()
            },
            Self::Import => Config {
                import_line_length: budget,
                ..Config::default()
            },
        }
    }

    /// The phrase naming this axis ahead of a width.
    fn label(self) -> &'static str {
        match self {
            Self::Code => "code width",
            Self::Docstring => "docstring width",
            Self::Fallback => "code width (import-line-length unset)",
            Self::Import => "import width",
        }
    }

    /// The [`AXES_VAR`] token naming this axis.
    fn name(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Docstring => "docstring",
            Self::Fallback => "fallback",
            Self::Import => "import",
        }
    }

    /// The command sweeping `path` alone on this axis at `width`.
    fn repro(self, width: usize, path: &Path) -> String {
        format!(
            "{CORPUS}={} {AXES_VAR}={} {WIDTHS_VAR}={width} cargo test --test corpus",
            path.display(),
            self.name(),
        )
    }
}

/// The defects one width's pass over the corpus found.
#[derive(Default)]
struct Findings {
    /// Runs that panicked, keyed by the message.
    panicked: Tally,
    /// Runs the pipeline rejected.
    rejected: Tally,
    /// Files the corpus held that the sweep could not read.
    skipped: usize,
    /// Outputs carrying a reported fix the run never applied.
    unapplied: Tally,
    /// Outputs a rule still rewrites.
    unsettled: Tally,
}

impl Findings {
    fn total(&self) -> usize {
        self.panicked.len() + self.rejected.len() + self.unapplied.len() + self.unsettled.len()
    }
}

impl Absorbing for Findings {
    fn absorb(&mut self, other: Self) {
        self.panicked.absorb(other.panicked);
        self.rejected.absorb(other.rejected);
        self.skipped += other.skipped;
        self.unapplied.absorb(other.unapplied);
        self.unsettled.absorb(other.unsettled);
    }
}

/// Every slice one sweep runs, with the checkpoint depths each must
/// record for the slices resuming behind it.
struct Plan {
    slices: Vec<Slice>,
    stops: Vec<Vec<usize>>,
}

impl Plan {
    /// Builds the slices `axes` and `widths` cross, the `code` axis
    /// leading so a budget-narrowed slice finds its trunk. A pipeline
    /// matching an earlier one drops as a duplicate, every other slice
    /// attaches behind the earliest earlier slice sharing its longest
    /// run of leading seat fingerprints, and the slice matching the
    /// shipped default keeps the lint pass.
    fn build(axes: &[Axis], widths: &[usize]) -> Self {
        let default_print = Pipeline::with_defaults(&Config::default()).fingerprint();
        let mut slices: Vec<Slice> = Vec::new();
        for &axis in axes {
            for &width in widths {
                let config = axis.config(width);
                let pipeline = Pipeline::with_defaults(&config);
                let prints = pipeline.fingerprints();
                if slices.iter().any(|held| held.prints == prints) {
                    continue;
                }
                let mut cut = 0;
                let mut parent = None;
                for (seat, held) in slices.iter().enumerate() {
                    let shared = held
                        .prints
                        .iter()
                        .zip(&prints)
                        .take_while(|(a, b)| a == b)
                        .count();
                    if shared > cut {
                        cut = shared;
                        parent = Some(seat);
                    }
                }
                debug_assert!(
                    parent.is_none_or(|seat| slices[seat].cut < cut),
                    "invariant: a slice resumes behind its parent's own entry",
                );
                slices.push(Slice {
                    axis,
                    cut,
                    lint: pipeline.fingerprint() == default_print,
                    parent,
                    pipeline,
                    prints,
                    width,
                });
            }
        }
        let mut stops = vec![BTreeSet::new(); slices.len()];
        for slice in &slices {
            if let Some(parent) = slice.parent {
                stops[parent].insert(slice.cut);
            }
        }
        Self {
            stops: stops
                .into_iter()
                .map(|depths| depths.into_iter().collect())
                .collect(),
            slices,
        }
    }
}

/// One fold of the sweep, entered behind the `cut` leading seats of
/// `parent`'s fold where one exists.
struct Slice {
    axis: Axis,
    cut: usize,
    lint: bool,
    parent: Option<usize>,
    pipeline: Pipeline,
    prints: Vec<String>,
    width: usize,
}

/// The axes this run sweeps, [`AXES_VAR`] narrowing [`Axis::ALL`] as a
/// space-separated list of `code`, `docstring`, `import`, and
/// `fallback`.
fn axes() -> Vec<Axis> {
    env_list(AXES_VAR, &Axis::ALL, |name| {
        *Axis::ALL
            .iter()
            .find(|axis| axis.name() == name)
            .unwrap_or_else(|| panic!("{AXES_VAR} names an unknown axis: {name}"))
    })
}

/// Runs one slice's fold over `entry` from seat `from`, records the
/// stop texts its dependents resume from, and files what the output
/// leaves behind, the run wrapped so a panic files against the file it
/// read. Returns the recorded stops, `None` where the fold failed.
fn probe(
    slice: &Slice,
    stops: &[usize],
    entry: Source,
    from: usize,
    path: &Path,
    findings: &mut Findings,
) -> Option<BTreeMap<usize, String>> {
    let hit = |detail: Option<String>| Hit {
        clause: Some((slice.axis.label().to_owned(), slice.width)),
        detail,
        repro: Some(slice.axis.repro(slice.width, path)),
    };
    let slot = Slot::open(format!(
        "{} at {}",
        path.display(),
        slice.axis.clause(slice.width)
    ));
    let recorded = RefCell::new(BTreeMap::new());
    let ran = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut current = entry;
        let mut opened = from;
        for &stop in stops {
            current = slice.pipeline.format_span(current, opened..stop)?;
            recorded
                .borrow_mut()
                .insert(stop, current.text().to_owned());
            opened = stop;
        }
        let formatted = slice
            .pipeline
            .format_span(current, opened..slice.prints.len())?;
        if slice.lint {
            let _ = slice.pipeline.diagnose(&formatted);
        }
        let report = slice.pipeline.settle_report(&formatted);
        Ok::<_, PipelineError>((formatted, report))
    }));
    drop(slot);
    let Ok(outcome) = ran else {
        let defect = PANIC
            .with(RefCell::take)
            .unwrap_or_else(|| "the run panicked".to_owned());
        findings.panicked.record_hit(defect, path, hit(None));
        return None;
    };
    let (
        formatted,
        SettleReport {
            editing,
            unlanded,
            witness,
        },
    ) = match outcome {
        Ok(pair) => pair,
        Err(error) => {
            findings
                .rejected
                .record_hit(format!("the run was rejected: {error}"), path, hit(None));
            return None;
        }
    };
    if verifying() {
        verify_resumed(slice, &formatted, path);
        verify_unlanded(&slice.pipeline, &formatted, &editing, &unlanded, path);
    }
    if !editing.is_empty() {
        let detail = witness.map(|(rule, second)| {
            excerpt(
                "formatted",
                &format!("`{rule}` on a second pass"),
                formatted.text(),
                &second,
            )
        });
        findings.unsettled.record_hit(
            format!("{} rewrites the output", render_slugs(&editing)),
            path,
            hit(detail),
        );
    }
    if !unlanded.is_empty() {
        findings.unapplied.record_hit(
            format!(
                "{} reports a fix the output never took",
                render_slugs(&unlanded)
            ),
            path,
            hit(None),
        );
    }
    Some(recorded.into_inner())
}

/// Sweeps every slice of `plan` over the file at `path`, each fold
/// resuming behind its parent's recorded text parsed under the file's
/// own name and a slice whose parent failed folding from the top on
/// its own.
fn sweep(plan: &Plan, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        findings.skipped += 1;
        return findings;
    };
    let name = path.display().to_string();
    let mut recorded: Vec<Option<BTreeMap<usize, String>>> = Vec::with_capacity(plan.slices.len());
    for (seat, slice) in plan.slices.iter().enumerate() {
        let (entry, from) = match slice.parent.map(|parent| &recorded[parent]) {
            Some(Some(stops)) => (
                Source::parse_named(stops[&slice.cut].clone(), &name)
                    .expect("invariant: a recorded checkpoint reparses"),
                slice.cut,
            ),
            _ => (source.clone(), 0),
        };
        recorded.push(probe(
            slice,
            &plan.stops[seat],
            entry,
            from,
            path,
            &mut findings,
        ));
    }
    findings
}

/// Formats `path` through the slice's whole fold as one run and panics
/// where the resumed fold's text differs.
fn verify_resumed(slice: &Slice, formatted: &Source, path: &Path) {
    let full = Source::from_path(path)
        .ok()
        .and_then(|source| slice.pipeline.format(source).ok());
    assert_eq!(
        full.as_ref().map(Source::text),
        Some(formatted.text()),
        "resumed fold differs at {} on {}",
        slice.axis.clause(slice.width),
        path.display(),
    );
    note_verified();
}

/// Reads the unlanded set off the diagnose pass and panics where it
/// differs from what `settle_report` read off one walk.
fn verify_unlanded(
    pipeline: &Pipeline,
    formatted: &Source,
    editing: &[RuleId],
    unlanded: &[RuleId],
    path: &Path,
) {
    let old_unlanded: Vec<_> = pipeline
        .diagnose(formatted)
        .into_iter()
        .filter(|d| d.severity == Severity::Format && d.fix.is_some())
        .map(|d| d.rule)
        .filter(|rule| !editing.contains(rule))
        .unique()
        .collect();
    assert_eq!(
        old_unlanded,
        unlanded,
        "unlanded set differs on {}",
        path.display()
    );
    note_verified();
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_width_settles_and_applies_what_it_reports() {
    let files = corpus();
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|info| {
        let at = info
            .location()
            .map_or_else(String::new, |site| format!(" at {site}"));
        let message = info.payload_as_str().unwrap_or("panicked");
        PANIC.with(|cell| cell.replace(Some(format!("the run panicked{at}: {message}"))));
    }));
    let axes = axes();
    let widths = widths_or(WIDTHS);
    let plan = Plan::build(&axes, &widths);
    eprintln!(
        "{} slices, {} resumed behind a parent, at {} widths on {} axes",
        plan.slices.len(),
        plan.slices
            .iter()
            .filter(|slice| slice.parent.is_some())
            .count(),
        widths.len(),
        axes.len(),
    );
    let findings = swept(&files, |path| sweep(&plan, path));
    panic::set_hook(previous);
    report_verified("probes against their reference runs");
    let unread = unread(findings.skipped, files.len(), "sweep");
    let report = format!(
        "{}{}{}{}",
        findings.panicked.render("runs that panicked"),
        findings.rejected.render("runs the pipeline rejected"),
        findings
            .unsettled
            .render("rewrites a second pass would change"),
        findings.unapplied.render("fixes the output never took"),
    );
    assert!(
        report.is_empty(),
        "{} distinct defects across {} files{unread} at {} widths on {} axes:{report}",
        findings.total(),
        files.len(),
        widths.len(),
        axes.len(),
    );
}

//! Corpus sweep at every configured line length: the text the formatter
//! writes leaves no rule rewriting it and no reported fix unapplied.
//! [`Pipeline::settle_report`] reads both defects off one walk over
//! every file's output. A run that panics or is rejected is recorded
//! against its file rather than ending the sweep, and a file passing
//! `BUDGET` stops the sweep and names itself. Each width in [`WIDTHS`]
//! runs once per axis in [`AXES`], one budget varied and the rest at
//! their defaults.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::Write,
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe},
    path::Path,
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use itertools::Itertools;
use prose::{
    config::Config,
    diagnostics::Severity,
    pipeline::{Pipeline, PipelineError, SettleReport},
    rule::{RuleId, render_slugs},
    source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use common::{
    CORPUS, EXCERPT, Hit, SHOWN, Tally, WIDTHS, WIDTHS_VAR, corpus, env_list, excerpt,
    note_verified, report_verified, verifying, widths_or,
};

mod common;

/// The axes the sweep crosses with every width absent [`AXES_VAR`],
/// each varying the budget it names.
const AXES: [Axis; 4] = [Axis::Code, Axis::Docstring, Axis::Fallback, Axis::Import];

/// The environment variable narrowing the axes by name.
const AXES_VAR: &str = "PROSE_SETTLE_AXES";

/// The wall clock one file may take before the sweep treats its run as
/// non-terminating.
const BUDGET: Duration = Duration::from_mins(1);

/// The files probes are reading right now, keyed by an opening order the
/// watchdog reads back.
static IN_FLIGHT: Mutex<BTreeMap<usize, (Instant, String)>> = Mutex::new(BTreeMap::new());

thread_local! {
    /// Where the silent hook last saw a panic raised, read back by the
    /// probe that caught it so a finding names the line as well as the
    /// message.
    static PANIC_SITE: RefCell<Option<String>> = const { RefCell::new(None) };
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
    /// Files the corpus held that the sweep could not read.
    skipped: usize,
    /// Runs the pipeline rejected.
    rejected: Tally,
    /// Outputs carrying a reported fix the run never applied.
    unapplied: Tally,
    /// Outputs a rule still rewrites.
    unsettled: Tally,
}

impl Findings {
    fn absorb(&mut self, other: Self) {
        self.panicked.absorb(other.panicked);
        self.skipped += other.skipped;
        self.rejected.absorb(other.rejected);
        self.unapplied.absorb(other.unapplied);
        self.unsettled.absorb(other.unsettled);
    }

    fn total(&self) -> usize {
        self.panicked.len() + self.rejected.len() + self.unapplied.len() + self.unsettled.len()
    }
}

/// One probe's entry in `IN_FLIGHT`, cleared however the probe leaves.
struct Slot(usize);

impl Slot {
    fn open(clause: &str, path: &Path) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let label = format!("{} at {clause}", path.display());
        registry().insert(id, (Instant::now(), label));
        Self(id)
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        registry().remove(&self.0);
    }
}

/// The axes this run sweeps, [`AXES_VAR`] narrowing [`AXES`] as a
/// space-separated list of `code`, `docstring`, `import`, and
/// `fallback`.
fn axes() -> Vec<Axis> {
    env_list(AXES_VAR, &AXES, |name| match name {
        "code" => Axis::Code,
        "docstring" => Axis::Docstring,
        "fallback" => Axis::Fallback,
        "import" => Axis::Import,
        other => panic!("{AXES_VAR} names an unknown axis: {other}"),
    })
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
fn probe(pipeline: &Pipeline, axis: Axis, width: usize, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        findings.skipped += 1;
        return findings;
    };
    let clause = axis.clause(width);
    let hit = |detail: Option<String>| Hit {
        clause: Some((axis.label().to_owned(), width)),
        detail,
        repro: Some(axis.repro(width, path)),
    };
    let slot = Slot::open(&clause, path);
    let ran = panic::catch_unwind(AssertUnwindSafe(|| {
        let (formatted, _) = pipeline.run(source)?;
        let report = pipeline.settle_report(&formatted);
        Ok::<_, PipelineError>((formatted, report))
    }));
    drop(slot);
    let outcome = match ran {
        Ok(outcome) => outcome,
        Err(payload) => {
            let site = PANIC_SITE.with(RefCell::take);
            let at = site.map_or_else(String::new, |site| format!(" at {site}"));
            findings.panicked.record_hit(
                format!("the run panicked{at}: {}", panic_message(&*payload)),
                path,
                hit(None),
            );
            return findings;
        }
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
            return findings;
        }
    };
    if verifying() {
        verify_unlanded(pipeline, &formatted, &editing, &unlanded, path);
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
    findings
}

/// The in-flight registry. The lock is never held across a run, so it
/// never carries a panic's poison.
fn registry() -> std::sync::MutexGuard<'static, BTreeMap<usize, (Instant, String)>> {
    IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner)
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

/// Ends the process once a probe outruns `BUDGET`, naming the file it
/// was reading. A rule that fails to terminate cannot be unwound from
/// the thread running it, and it grows until the machine reaches an
/// out-of-memory kill, so the sweep stops itself first.
fn watch_for_a_runaway() {
    thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(1));
            let overrun = registry()
                .values()
                .find(|(since, _)| since.elapsed() > BUDGET)
                .map(|(_, label)| label.clone());
            if let Some(label) = overrun {
                let mut stderr = std::io::stderr();
                let _ = writeln!(
                    stderr,
                    "the sweep stopped itself, since {label} has run past {} seconds and is \
                     treated as non-terminating",
                    BUDGET.as_secs(),
                );
                let _ = stderr.flush();
                std::process::exit(101);
            }
        }
    });
}

#[test]
#[cfg_attr(coverage, ignore = "the sweep runs uninstrumented in its own row")]
fn every_width_settles_and_applies_what_it_reports() {
    let files = corpus();
    assert!(!files.is_empty(), "the corpus holds no `.py` files");
    watch_for_a_runaway();
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|info| {
        let site = info
            .location()
            .map(|at| format!("{}:{}", at.file(), at.line()));
        PANIC_SITE.with(|cell| cell.replace(site));
    }));
    let axes = axes();
    let widths = widths_or(&WIDTHS);
    let mut findings = Findings::default();
    for &width in &widths {
        for &axis in &axes {
            let pipeline = Pipeline::with_defaults(&axis.config(width));
            findings.absorb(
                files
                    .par_iter()
                    .map(|path| probe(&pipeline, axis, width, path))
                    .reduce(Findings::default, |mut held, next| {
                        held.absorb(next);
                        held
                    }),
            );
        }
    }
    panic::set_hook(previous);
    report_verified("settle reports against the diagnose pass");
    // Every configuration walks the same file list, so the skips divide
    // back to the count of files no configuration could read.
    let unreadable = findings.skipped / (widths.len() * axes.len());
    let unread = match unreadable {
        0 => String::new(),
        n => {
            let mut stderr = std::io::stderr();
            let _ = writeln!(
                stderr,
                "the sweep could not read {n} of the {} files under the corpus root",
                files.len()
            );
            let _ = stderr.flush();
            format!(" and {n} the sweep could not read")
        }
    };
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

#[test]
fn excerpt_counts_both_the_lines_and_the_hunks_past_its_cap() {
    let before: String = (1..=60).map(|n| format!("line {n}\n")).collect();
    let after: String = (1..=60)
        .map(|n| {
            if n <= 30 || n == 55 {
                format!("row {n}\n")
            } else {
                format!("line {n}\n")
            }
        })
        .collect();

    let shown = excerpt("before", "after", &before, &after);

    assert!(shown.ends_with(" more lines and 1 more hunks"), "{shown}");
}

#[test]
fn excerpt_counts_the_lines_past_its_cap() {
    let before: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let after: String = (1..=40).map(|n| format!("row {n}\n")).collect();

    let shown = excerpt("before", "after", &before, &after);

    assert_eq!(shown.lines().count(), 2 + EXCERPT + 1, "{shown}");
    assert!(shown.ends_with(" more lines"), "{shown}");
}

#[test]
fn excerpt_ends_on_the_hunk_when_nothing_is_cut() {
    let before: String = (1..=10).map(|n| format!("line {n}\n")).collect();
    let after = before.replace("line 2\n", "line two\n");

    let shown = excerpt("before", "after", &before, &after);

    assert!(!shown.contains("..."), "{shown}");
    assert_eq!(shown.lines().count(), 2 + 7, "{shown}");
}

#[test]
fn excerpt_shows_the_first_hunk_and_counts_the_rest() {
    let before: String = (1..=60).map(|n| format!("line {n}\n")).collect();
    let after = before
        .replace("line 2\n", "line two\n")
        .replace("line 50\n", "line fifty\n");

    let shown = excerpt("first pass", "second pass", &before, &after);

    assert!(
        shown.starts_with("--- first pass\n+++ second pass\n@@"),
        "{shown}"
    );
    assert!(shown.contains("-line 2\n+line two\n"), "{shown}");
    assert!(shown.ends_with("... and 1 more hunks"), "{shown}");
}

#[test]
fn tally_names_a_defect_once_across_clauses_and_keeps_the_earliest_example() {
    let hit = |label: &str, width| Hit {
        clause: Some((label.to_owned(), width)),
        detail: None,
        repro: None,
    };
    let mut tally = Tally::default();
    tally.record_hit(
        "still editing".to_owned(),
        Path::new("b.py"),
        hit("code", 88),
    );
    tally.record_hit(
        "still editing".to_owned(),
        Path::new("a.py"),
        hit("import", 60),
    );
    tally.record_hit(
        "still editing".to_owned(),
        Path::new("a.py"),
        hit("code", 40),
    );

    let rendered = tally.render("defects");

    assert_eq!(tally.len(), 1);
    assert!(
        rendered.contains("still editing (2 files, e.g. a.py at code 40)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("reached at code 40, 88 and import 60"),
        "{rendered}"
    );
}

#[test]
fn tally_render_caps_the_defects_it_prints_and_counts_the_rest() {
    let mut tally = Tally::default();
    for n in 0..=SHOWN {
        let hit = Hit {
            clause: None,
            detail: None,
            repro: None,
        };
        tally.record_hit(format!("defect {n:02}"), Path::new("a.py"), hit);
    }

    let rendered = tally.render("defects");

    assert!(
        rendered.contains(&format!("defect {:02}", SHOWN - 1)),
        "{rendered}"
    );
    assert!(
        !rendered.contains(&format!("defect {SHOWN:02}")),
        "{rendered}"
    );
    assert!(rendered.ends_with("... and 1 more"), "{rendered}");
}

#[test]
fn tally_render_carries_the_example_repro_and_detail() {
    let mut tally = Tally::default();
    let hit = Hit {
        clause: None,
        detail: Some("--- a\n+++ b".to_owned()),
        repro: Some("cargo test".to_owned()),
    };
    tally.record_hit("still editing".to_owned(), Path::new("a.py"), hit);

    let rendered = tally.render("defects");

    assert!(
        rendered.contains("still editing (1 file, e.g. a.py)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("\n    reproduce with cargo test"),
        "{rendered}"
    );
    assert!(rendered.ends_with("\n    --- a\n    +++ b"), "{rendered}");
}

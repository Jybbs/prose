//! Corpus sweep at every configured line length: the text the formatter
//! writes leaves no rule rewriting it and no reported fix unapplied.
//! [`Pipeline::unsettled`] reports the first defect and the diagnostic
//! pass alone reports the second, so the sweep reads both over every
//! file. A run that panics or is rejected is recorded against its file
//! rather than ending the sweep, and a file passing `BUDGET` stops the
//! sweep and names itself. Each width in [`WIDTHS`] runs once per axis
//! in [`AXES`], one budget varied and the rest at their defaults.
//! `PROSE_SETTLE_CORPUS` points the sweep at another directory,
//! `PROSE_SETTLE_WIDTHS` overrides the width set, and
//! `PROSE_SETTLE_AXES` narrows the axes by name.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    env,
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
    pipeline::{Pipeline, PipelineError},
    rule::render_slugs,
    source::Source,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use common::{Tally, corpus};

mod common;

/// The axes the sweep crosses with every width absent
/// `PROSE_SETTLE_AXES`, each varying the budget it names.
const AXES: [Axis; 4] = [Axis::Code, Axis::Docstring, Axis::Fallback, Axis::Import];

/// The line lengths the sweep covers absent `PROSE_SETTLE_WIDTHS`, the
/// default flanked by the narrow and wide settings a project sets it
/// to.
const WIDTHS: [usize; 6] = [40, 50, 60, 79, 88, 100];

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
        match self {
            Self::Code => format!("code width {width}"),
            Self::Docstring => format!("docstring width {width}"),
            Self::Fallback => format!("code width {width} with import-line-length unset"),
            Self::Import => format!("import width {width}"),
        }
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

    /// The `PROSE_SETTLE_AXES` token naming this axis.
    fn name(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Docstring => "docstring",
            Self::Fallback => "fallback",
            Self::Import => "import",
        }
    }

    /// The command sweeping this axis at `width` alone, carrying the
    /// corpus override when the run took one.
    fn repro(self, width: usize) -> String {
        let corpus = env::var("PROSE_SETTLE_CORPUS").map_or_else(
            |_| String::new(),
            |dir| format!("PROSE_SETTLE_CORPUS={dir} "),
        );
        format!(
            "{corpus}PROSE_SETTLE_AXES={} PROSE_SETTLE_WIDTHS={width} cargo test --test corpus",
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

/// The wall clock one file may take before the sweep treats its run as
/// non-terminating.
const BUDGET: Duration = Duration::from_mins(1);

/// The files probes are reading right now, keyed by an opening order the
/// watchdog reads back.
static IN_FLIGHT: Mutex<BTreeMap<usize, (Instant, String)>> = Mutex::new(BTreeMap::new());

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

thread_local! {
    /// Where the silent hook last saw a panic raised, read back by the
    /// probe that caught it so a finding names the line as well as the
    /// message.
    static PANIC_SITE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The axes this run sweeps, `PROSE_SETTLE_AXES` narrowing [`AXES`] as
/// a space-separated list of `code`, `docstring`, `import`, and
/// `fallback`.
fn axes() -> Vec<Axis> {
    env_list("PROSE_SETTLE_AXES", &AXES, |name| match name {
        "code" => Axis::Code,
        "docstring" => Axis::Docstring,
        "fallback" => Axis::Fallback,
        "import" => Axis::Import,
        other => panic!("PROSE_SETTLE_AXES names an unknown axis: {other}"),
    })
}

/// The values `var` carries as a space-separated list, `defaults`
/// where it is unset.
fn env_list<T: Clone>(var: &str, defaults: &[T], parse: impl Fn(&str) -> T) -> Vec<T> {
    env::var(var).map_or_else(
        |_| defaults.to_vec(),
        |set| set.split_whitespace().map(&parse).collect(),
    )
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
fn probe(pipeline: &Pipeline, clause: &str, repro: &str, path: &Path) -> Findings {
    let mut findings = Findings::default();
    let Ok(source) = Source::from_path(path) else {
        findings.skipped += 1;
        return findings;
    };
    let slot = Slot::open(clause, path);
    let ran = panic::catch_unwind(AssertUnwindSafe(|| {
        let (formatted, _) = pipeline.run(source)?;
        let editing = pipeline.unsettled(&formatted);
        let unlanded: Vec<_> = pipeline
            .diagnose(&formatted)
            .into_iter()
            .filter(|d| d.severity == Severity::Format && d.fix.is_some())
            .map(|d| d.rule)
            .filter(|rule| !editing.contains(rule))
            .unique()
            .collect();
        Ok::<_, PipelineError>((editing, unlanded))
    }));
    drop(slot);
    let outcome = match ran {
        Ok(outcome) => outcome,
        Err(payload) => {
            let site = PANIC_SITE.with(RefCell::take);
            let at = site.map_or_else(String::new, |site| format!(" at {site}"));
            findings.panicked.record_at(
                format!(
                    "at {clause}, the run panicked{at}: {}",
                    panic_message(&*payload)
                ),
                path,
                Some(repro),
            );
            return findings;
        }
    };
    let (editing, unlanded) = match outcome {
        Ok(pair) => pair,
        Err(error) => {
            findings.rejected.record_at(
                format!("at {clause}, the run was rejected: {error}"),
                path,
                Some(repro),
            );
            return findings;
        }
    };
    if !editing.is_empty() {
        findings.unsettled.record_at(
            format!(
                "at {clause}, {} rewrites the output",
                render_slugs(&editing)
            ),
            path,
            Some(repro),
        );
    }
    if !unlanded.is_empty() {
        findings.unapplied.record_at(
            format!(
                "at {clause}, {} reports a fix the output never took",
                render_slugs(&unlanded)
            ),
            path,
            Some(repro),
        );
    }
    findings
}

/// The in-flight registry. The lock is never held across a run, so it
/// never carries a panic's poison.
fn registry() -> std::sync::MutexGuard<'static, BTreeMap<usize, (Instant, String)>> {
    IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner)
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

/// The line lengths this run sweeps, `PROSE_SETTLE_WIDTHS` overriding
/// [`WIDTHS`] as a space-separated list.
fn widths() -> Vec<usize> {
    env_list("PROSE_SETTLE_WIDTHS", &WIDTHS, |width| {
        width
            .parse::<NonZeroUsize>()
            .expect("every `PROSE_SETTLE_WIDTHS` entry is a nonzero number")
            .get()
    })
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
    let widths = widths();
    let mut findings = Findings::default();
    for &width in &widths {
        for &axis in &axes {
            let pipeline = Pipeline::with_defaults(&axis.config(width));
            let clause = axis.clause(width);
            let repro = axis.repro(width);
            findings.absorb(
                files
                    .par_iter()
                    .map(|path| probe(&pipeline, &clause, &repro, path))
                    .reduce(Findings::default, |mut held, next| {
                        held.absorb(next);
                        held
                    }),
            );
        }
    }
    panic::set_hook(previous);
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

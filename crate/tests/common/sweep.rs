//! The corpus-sweep surface the settle and subset probes share.

use std::{
    cmp::Reverse,
    collections::BTreeMap,
    env,
    io::{self, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process,
    sync::{
        Mutex, MutexGuard, Once, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use ignore::WalkBuilder;
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

/// The wall clock one probe may take before a sweep treats its run as
/// non-terminating.
const BUDGET: Duration = Duration::from_mins(1);

/// The environment variable aiming a sweep at a directory other than
/// the fixture tree.
pub(crate) const CORPUS: &str = "PROSE_SETTLE_CORPUS";

/// The probes reading right now, keyed by an opening order the watchdog
/// reads back.
static IN_FLIGHT: Mutex<BTreeMap<usize, (Instant, String)>> = Mutex::new(BTreeMap::new());

/// How many probes this run has verified against their reference
/// readings.
static VERIFIED: AtomicUsize = AtomicUsize::new(0);

/// The environment variable that folds every probe beside its
/// reference reading and fails on a divergence.
const VERIFY_VAR: &str = "PROSE_SETTLE_VERIFY";

/// The line lengths a sweep covers absent [`WIDTHS_VAR`], the shipped
/// default flanked by the narrow and wide settings a project sets it
/// to.
pub(crate) const WIDTHS: &[usize] = &[40, 50, 60, 79, 88, 100];

/// The environment variable naming the line lengths a sweep covers.
pub(crate) const WIDTHS_VAR: &str = "PROSE_SETTLE_WIDTHS";

/// A per-file finding set a parallel sweep folds together.
pub(crate) trait Absorbing: Default + Send {
    /// Folds `other`'s findings into this set.
    fn absorb(&mut self, other: Self);
}

/// One probe's entry in [`IN_FLIGHT`], cleared however the probe leaves.
pub(crate) struct Slot(usize);

impl Slot {
    /// Opens an entry the watchdog names by `label` should the probe
    /// outrun [`BUDGET`].
    pub(crate) fn open(label: String) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        registry().insert(id, (Instant::now(), label));
        Self(id)
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        registry().remove(&self.0);
    }
}

/// The `.py` files a sweep reads, with the runaway watchdog opened and
/// an empty corpus failing the sweep outright.
pub(crate) fn corpus() -> Vec<PathBuf> {
    let files = walked();
    assert!(!files.is_empty(), "the corpus holds no `.py` files");
    watch_for_a_runaway();
    files
}

/// The values `var` carries as a space-separated list, `defaults` where
/// it is unset.
pub(crate) fn env_list<T: Clone>(var: &str, defaults: &[T], parse: impl Fn(&str) -> T) -> Vec<T> {
    setting(var).map_or_else(|| defaults.to_vec(), |set| env_list_of(&set, parse))
}

/// The values `set` lists, separated by spaces or commas, each read
/// through `parse`.
pub(crate) fn env_list_of<T>(set: &str, parse: impl Fn(&str) -> T) -> Vec<T> {
    set.split([' ', ','])
        .filter(|part| !part.is_empty())
        .map(parse)
        .collect()
}

/// Counts one probe verified against its reference reading.
pub(crate) fn note_verified() {
    VERIFIED.fetch_add(1, Ordering::Relaxed);
}

/// The directory [`CORPUS`] aims a sweep at, `None` for the fixture
/// tree.
pub(crate) fn pointed_corpus() -> Option<PathBuf> {
    setting(CORPUS).map(PathBuf::from)
}

/// Prints how many probes were verified, `what` naming them, when any
/// were.
pub(crate) fn report_verified(what: &str) {
    let verified = VERIFIED.load(Ordering::Relaxed);
    if verified > 0 {
        eprintln!("verified {verified} {what}");
    }
}

/// The `.py` files under `root`. The walk carries no standard filter, so a
/// hidden directory and an ignored one both enter the sweep rather than
/// leaving it short without saying so.
pub(crate) fn python_files(root: &Path) -> impl Iterator<Item = PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
}

/// The value `var` carries, `None` where it is unset or blank.
pub(crate) fn setting(var: &str) -> Option<String> {
    env::var(var).ok().filter(|value| !value.trim().is_empty())
}

/// Folds what `probe` reads off each of `files`, walked in parallel.
pub(crate) fn swept<F: Absorbing>(
    files: &[PathBuf],
    probe: impl Fn(&Path) -> F + Send + Sync,
) -> F {
    files
        .par_iter()
        .map(|path| probe(path))
        .reduce(F::default, |mut held, next| {
            held.absorb(next);
            held
        })
}

/// The clause a report carries for the files a sweep could not read,
/// naming the count on stderr as it goes.
pub(crate) fn unread(count: usize, total: usize, what: &str) -> String {
    if count == 0 {
        return String::new();
    }
    eprintln!("the {what} could not read {count} of the {total} files under the corpus root");
    format!(" and {count} the {what} could not read")
}

/// Whether [`VERIFY_VAR`] is set.
pub(crate) fn verifying() -> bool {
    setting(VERIFY_VAR).is_some()
}

/// Ends the process once a probe outruns [`BUDGET`], naming what it was
/// reading. A rule that fails to terminate cannot be unwound from the
/// thread running it, and it grows until the machine reaches an
/// out-of-memory kill, so a sweep stops itself first. The watch starts
/// once however many times it is called.
pub(crate) fn watch_for_a_runaway() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_secs(1));
                let overrun = registry()
                    .values()
                    .find(|(since, _)| since.elapsed() > BUDGET)
                    .map(|(_, label)| label.clone());
                if let Some(label) = overrun {
                    let mut stderr = io::stderr();
                    let _ = writeln!(
                        stderr,
                        "the sweep stopped itself, since {label} has run past {} seconds and is \
                         treated as non-terminating",
                        BUDGET.as_secs(),
                    );
                    let _ = stderr.flush();
                    process::exit(101);
                }
            }
        });
    });
}

/// The line lengths this run sweeps, [`WIDTHS_VAR`] overriding
/// `defaults` as a space-separated list.
pub(crate) fn widths_or(defaults: &[usize]) -> Vec<usize> {
    env_list(WIDTHS_VAR, defaults, |width| {
        width
            .parse::<NonZeroUsize>()
            .unwrap_or_else(|_| panic!("every `{WIDTHS_VAR}` entry is a nonzero number"))
            .get()
    })
}

/// The in-flight registry. The lock is never held across a run, so it
/// never carries a panic's poison.
fn registry() -> MutexGuard<'static, BTreeMap<usize, (Instant, String)>> {
    IN_FLIGHT.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The `.py` files under the corpus root, largest first with the path
/// breaking ties, so a parallel sweep's tail is one file long and a
/// failure names the same file across runs. [`CORPUS`] points a sweep
/// at a directory other than the fixture tree.
fn walked() -> Vec<PathBuf> {
    let root = pointed_corpus()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    python_files(&root)
        .sorted_by_cached_key(|path| {
            let size = fs_err::metadata(path).map_or(0, |data| data.len());
            (Reverse(size), path.clone())
        })
        .collect()
}

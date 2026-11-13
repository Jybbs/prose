//! The corpus-sweep surface the settle, subset, and import probes share.

use std::{
    cmp::Reverse,
    collections::BTreeMap,
    env,
    io::{self, Write},
    mem::DropGuard,
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
use ruff_python_ast::{PySourceType, PythonVersion};

/// The wall clock one probe may take before a sweep treats its run as
/// non-terminating.
const BUDGET: Duration = Duration::from_mins(1);

/// The environment variable aiming a sweep at a directory other than
/// the fixture tree.
const CORPUS: &str = "PROSE_SETTLE_CORPUS";

/// The probes reading right now, keyed by an opening order the watchdog
/// reads back.
static IN_FLIGHT: Mutex<BTreeMap<usize, (Instant, String)>> = Mutex::new(BTreeMap::new());

/// The `target-version` settings a sweep covers absent [`TARGETS_VAR`],
/// the shipped default of none beside the newest stable version the
/// parser models, which sits at or above every rule's version floor.
pub const TARGETS: &[Option<PythonVersion>] = &[None, Some(PythonVersion::latest())];

/// The environment variable naming the `target-version` settings a sweep
/// covers, [`UNSET`] standing for the shipped default.
const TARGETS_VAR: &str = "PROSE_SETTLE_TARGETS";

/// The [`TARGETS_VAR`] entry naming an unset `target-version`.
const UNSET: &str = "unset";

/// How many probes this run has verified against their reference
/// readings.
static VERIFIED: AtomicUsize = AtomicUsize::new(0);

/// The environment variable that folds every probe beside its
/// reference reading and fails on a divergence.
const VERIFY_VAR: &str = "PROSE_SETTLE_VERIFY";

/// The line lengths a sweep covers absent [`WIDTHS_VAR`], the shipped
/// default flanked by the narrow and wide settings a project sets it
/// to.
pub const WIDTHS: &[usize] = &[40, 50, 60, 79, 88, 100];

/// The environment variable naming the line lengths a sweep covers.
pub const WIDTHS_VAR: &str = "PROSE_SETTLE_WIDTHS";

/// A per-file finding set a parallel sweep folds together.
pub trait Absorbing: Default + Send {
    /// Folds `other`'s findings into this set.
    fn absorb(&mut self, other: Self);
}

/// The Python sources a sweep reads, under the directory [`CORPUS`] names or
/// under `fixtures` where it is unset, with the runaway watchdog opened
/// and an empty corpus failing the sweep outright.
pub fn corpus(fixtures: &Path) -> Vec<PathBuf> {
    let files = walked(fixtures);
    assert!(!files.is_empty(), "the corpus holds no Python sources");
    watch_for_a_runaway();
    files
}

/// The values `var` carries as a space-separated list, `defaults` where
/// it is unset.
pub fn env_list<T: Clone>(var: &str, defaults: &[T], parse: impl Fn(&str) -> T) -> Vec<T> {
    setting(var).map_or_else(|| defaults.to_vec(), |set| env_list_of(&set, parse))
}

/// The values `set` lists, separated by spaces or commas, each read
/// through `parse`.
pub fn env_list_of<T>(set: &str, parse: impl Fn(&str) -> T) -> Vec<T> {
    set.split([' ', ','])
        .filter(|part| !part.is_empty())
        .map(parse)
        .collect()
}

/// Counts one probe verified against its reference reading.
pub fn note_verified() {
    VERIFIED.fetch_add(1, Ordering::Relaxed);
}

/// Opens the probe's entry in [`IN_FLIGHT`] under `label`, the name the
/// watchdog prints should the probe outrun [`BUDGET`]. The returned guard
/// clears the entry however the probe leaves.
pub fn open_slot(label: String) -> DropGuard<usize, impl FnOnce(usize)> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    registry().insert(id, (Instant::now(), label));
    DropGuard::new(id, |id| {
        registry().remove(&id);
    })
}

/// The directory [`CORPUS`] aims a sweep at, `None` for the fixture
/// tree.
pub fn pointed_corpus() -> Option<PathBuf> {
    setting(CORPUS).map(PathBuf::from)
}

/// The Python sources under `root`, covering the `.py`, `.pyw`, and `.pyi`
/// files the walker formats and leaving notebooks to the surface that reads
/// a `SourceMap`. The walk carries no standard filter, so a hidden directory
/// and an ignored one both enter the sweep rather than leaving it short
/// without saying so.
pub fn python_files(root: &Path) -> impl Iterator<Item = PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| {
            PySourceType::try_from_path(path).is_some_and(PySourceType::is_py_file_or_stub)
        })
}

/// Prints how many probes were verified, `what` naming them, when any
/// were.
pub fn report_verified(what: &str) {
    let verified = VERIFIED.load(Ordering::Relaxed);
    if verified > 0 {
        eprintln!("verified {verified} {what}");
    }
}

/// The command sweeping `path` alone through the `test` binary at
/// `target` and `width`, `narrowing` naming any further setting ahead of
/// them.
pub fn repro_command(
    test: &str,
    path: &Path,
    narrowing: &str,
    target: Option<PythonVersion>,
    width: usize,
) -> String {
    format!(
        "{CORPUS}={} {narrowing}{TARGETS_VAR}={} {WIDTHS_VAR}={width} cargo test --test {test}",
        path.display(),
        target_name(target),
    )
}

/// The value `var` carries, `None` where it is unset or blank.
pub fn setting(var: &str) -> Option<String> {
    env::var(var).ok().filter(|value| !value.trim().is_empty())
}

/// Folds what `probe` reads off each of `files`, walked in parallel.
pub fn swept<F: Absorbing>(files: &[PathBuf], probe: impl Fn(&Path) -> F + Send + Sync) -> F {
    files
        .par_iter()
        .map(|path| probe(path))
        .reduce(F::default, |mut held, next| {
            held.absorb(next);
            held
        })
}

/// The [`TARGETS_VAR`] entry naming `target`, which a report and a
/// reproduction command both print.
pub fn target_name(target: Option<PythonVersion>) -> String {
    target.map_or_else(|| UNSET.to_owned(), |version| version.to_string())
}

/// The [`target_name`] of each of `targets`, comma-separated, which a
/// sweep's summary and its failure report both print.
pub fn target_names(targets: &[Option<PythonVersion>]) -> String {
    targets.iter().copied().map(target_name).join(", ")
}

/// The `target-version` settings this run sweeps, [`TARGETS_VAR`]
/// overriding `defaults` as a space-separated list of `major.minor`
/// versions and [`UNSET`].
pub fn targets_or(defaults: &[Option<PythonVersion>]) -> Vec<Option<PythonVersion>> {
    env_list(TARGETS_VAR, defaults, target_of)
}

/// The clause a report carries for the files a sweep could not read,
/// naming the count on stderr as it goes.
pub fn unread(count: usize, total: usize, what: &str) -> String {
    if count == 0 {
        return String::new();
    }
    eprintln!("the {what} could not read {count} of the {total} files under the corpus root");
    format!(" and {count} the {what} could not read")
}

/// Whether [`VERIFY_VAR`] is set.
pub fn verifying() -> bool {
    setting(VERIFY_VAR).is_some()
}

/// Ends the process once a probe outruns [`BUDGET`], naming what it was
/// reading. A rule that fails to terminate cannot be unwound from the
/// thread running it, and it grows until the machine reaches an
/// out-of-memory kill, so a sweep stops itself first. The watch starts
/// once however many times it is called.
pub fn watch_for_a_runaway() {
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
pub fn widths_or(defaults: &[usize]) -> Vec<usize> {
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

/// The `target-version` a [`TARGETS_VAR`] entry names, [`UNSET`] reading
/// as none.
fn target_of(token: &str) -> Option<PythonVersion> {
    (token != UNSET).then(|| {
        token.parse().unwrap_or_else(|error| {
            panic!("every `{TARGETS_VAR}` entry is `{UNSET}` or a version: {error}")
        })
    })
}

/// The Python sources under the corpus root, largest first with the path
/// breaking ties, so a parallel sweep's tail is one file long and a
/// failure names the same file across runs. [`CORPUS`] points a sweep
/// at a directory other than `fixtures`.
fn walked(fixtures: &Path) -> Vec<PathBuf> {
    let root = pointed_corpus().unwrap_or_else(|| fixtures.to_path_buf());
    python_files(&root)
        .sorted_by_cached_key(|path| {
            let size = fs_err::metadata(path).map_or(0, |data| data.len());
            (Reverse(size), path.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_python_ast::PythonVersion;

    use super::*;

    #[test]
    fn open_slot_clears_its_entry_once_dropped() {
        let slot = open_slot("a probe".to_string());
        let id = *slot;
        assert!(registry().contains_key(&id));
        drop(slot);
        assert!(!registry().contains_key(&id));
    }

    #[rstest]
    #[case::unset(None)]
    #[case::a_version_floor(Some(PythonVersion::PY39))]
    #[case::the_newest_version(Some(PythonVersion::latest()))]
    fn target_of_reads_back_the_entry_target_name_prints(#[case] target: Option<PythonVersion>) {
        assert_eq!(target_of(&target_name(target)), target);
    }

    #[rstest]
    #[should_panic(expected = "is `unset` or a version")]
    fn target_of_rejects_an_entry_other_than_unset_or_major_minor(
        #[values("", "3", "3.14.6", "py314", "Unset")] entry: &str,
    ) {
        let _ = target_of(entry);
    }

    #[rstest]
    #[case(0, "")]
    #[case(3, " and 3 the probe could not read")]
    fn unread_names_the_count_only_where_a_file_went_unread(
        #[case] count: usize,
        #[case] clause: &str,
    ) {
        assert_eq!(unread(count, 1050, "probe"), clause);
    }
}

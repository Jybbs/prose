//! The on-disk cache store: directory walk, atomic insert, and
//! LRU eviction.

use std::{
    fs::Metadata,
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};

use fs_err::DirEntry;
use rustc_hash::FxHashSet;
use tempfile::NamedTempFile;

use super::{CacheEntry, CacheEntryRef, CacheInfo, CacheKey, CleanReport, key::generation};

/// How long a generation directory must sit untouched before a sweep
/// reclaims it.
const GENERATION_GRACE: Duration = Duration::from_hours(1);

/// The fraction of each cap an eviction pass drives down to, as a
/// multiplier over a divisor.
const LOW_WATER_DIVISOR: u64 = 5;

const LOW_WATER_MULTIPLIER: u64 = 4;

/// How far an entry's recorded mtime may lag the clock before a hit
/// rewrites it.
const MTIME_GRANULARITY: Duration = Duration::from_hours(1);

/// User-level on-disk cache.
#[derive(Debug)]
pub struct Cache {
    /// Set by the first [`insert`](Self::insert) to land, read by the
    /// run's closing sweep.
    pub(super) inserted: AtomicBool,
    pub(super) max_entries: usize,
    pub(super) max_size_bytes: u64,
    /// The own-output marker keys, loaded from the generation's ledger
    /// on first probe. A marked key names bytes a write-back run
    /// landed, so a later run rewriting them has found a settle
    /// defect. Advisory only, in that a lost marker loses a detection
    /// and never corrupts a run.
    pub(super) own_output: OnceLock<FxHashSet<[u8; 32]>>,
    /// This build's generation directory, where every entry lands.
    pub(super) root: PathBuf,
    /// The directory holding every generation, swept for dead ones.
    pub(super) store: PathBuf,
}

impl Cache {
    /// An uncapped cache rooted under `store`, one generation deep.
    #[cfg(test)]
    pub(crate) fn in_store(store: PathBuf) -> Self {
        let root = store.join("generation");
        fs_err::create_dir_all(&root).expect("creates the cache root");
        Self::rooted(store, root)
    }

    /// Sets the LRU eviction cap on the directory's entry count.
    #[must_use]
    pub fn with_max_entries(mut self, entries: u32) -> Self {
        self.max_entries = entries as usize;
        self
    }

    /// Sets the LRU eviction cap in MiB.
    #[must_use]
    pub fn with_max_size_mib(mut self, mib: u32) -> Self {
        self.max_size_bytes = u64::from(mib) * 1024 * 1024;
        self
    }

    /// True where an eviction pass has driven the directory far enough
    /// below its caps to stop. The entry floor holds at one, so a cap
    /// of one still keeps an entry.
    fn below_low_water(&self, total: u64, count: usize) -> bool {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.max_entries).unwrap_or(u64::MAX);
        total <= low_water(self.max_size_bytes) && count <= low_water(entries).max(1)
    }

    fn entries(&self) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
        entries_in(&self.root)
    }

    fn own_output_path(&self) -> PathBuf {
        self.root.join("own-output")
    }

    /// Removes every generation directory but this build's, plus any
    /// entry file left directly in the store by a build that predates
    /// the generation layout. A directory touched within
    /// [`GENERATION_GRACE`] is left alone.
    fn prune_dead_generations(&self) -> CleanReport {
        let mut report = CleanReport::default();
        let now = SystemTime::now();
        for entry in dir_entries(&self.store) {
            let path = entry.path();
            if path == self.root {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_dir() {
                if is_entry_file(&path) && fs_err::remove_file(&path).is_ok() {
                    report.record(on_disk(&metadata));
                }
                continue;
            }
            if metadata
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok())
                .is_none_or(|age| age < GENERATION_GRACE)
            {
                continue;
            }
            sweep_dir(&mut report, &path);
        }
        report
    }

    /// An uncapped cache rooted at `root` under `store`.
    fn rooted(store: PathBuf, root: PathBuf) -> Self {
        Self {
            inserted: AtomicBool::new(false),
            max_entries: usize::MAX,
            max_size_bytes: u64::MAX,
            own_output: OnceLock::new(),
            root,
            store,
        }
    }

    /// Every entry the store holds, across this build's generation and
    /// any older one a sweep has not yet reclaimed.
    fn stored(&self) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
        let generations: Vec<PathBuf> = dir_entries(&self.store)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path())
            .collect();
        entries_in(&self.store).chain(generations.into_iter().flat_map(|d| entries_in(&d)))
    }

    fn try_insert(&self, key: &CacheKey, value: &CacheEntryRef<'_>) -> io::Result<()> {
        let mut tmp = NamedTempFile::with_suffix_in(".tmp", &self.root)?;
        {
            let mut buf = BufWriter::new(tmp.as_file_mut());
            postcard::to_io(value, &mut buf).map_err(io::Error::other)?;
            buf.flush()?;
        }
        tmp.persist(self.path_for(key)).map_err(|e| e.error)?;
        Ok(())
    }

    /// True where a directory of `count` entries occupying `total` bytes
    /// sits inside both configured caps.
    fn within_caps(&self, total: u64, count: usize) -> bool {
        total <= self.max_size_bytes && count <= self.max_entries
    }

    /// Removes every file in the cache directory and returns the
    /// count and freed bytes, including any orphan sidecars or stray
    /// files alongside the keyed entries.
    ///
    /// # Errors
    ///
    /// Returns the underlying IO error if the cache directory cannot
    /// be read.
    pub fn clean(&self) -> io::Result<CleanReport> {
        let mut report = CleanReport::default();
        for entry in fs_err::read_dir(&self.store)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                sweep_dir(&mut report, &path);
                continue;
            }
            remove_into(&mut report, &entry);
        }
        fs_err::create_dir_all(&self.root)?;
        Ok(report)
    }

    /// Runs the LRU eviction pass to honor the configured size and
    /// entry-count caps, and returns what it removed. Each entry counts
    /// the blocks the filesystem allocated to it.
    pub fn compact(&self) -> CleanReport {
        let mut report = self.prune_dead_generations();
        let mut files: Vec<(SystemTime, u64, DirEntry)> = self
            .entries()
            .map(|(e, m)| {
                (
                    m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    on_disk(&m),
                    e,
                )
            })
            .collect();
        let mut total: u64 = files.iter().map(|(_, bytes, _)| *bytes).sum();
        let mut count = files.len();
        if self.within_caps(total, count) {
            return report;
        }
        files.sort_by_key(|(mtime, _, _)| *mtime);
        for (_, bytes, entry) in files {
            if self.below_low_water(total, count) {
                break;
            }
            match fs_err::remove_file(entry.path()) {
                Ok(()) => {
                    total = total.saturating_sub(bytes);
                    count -= 1;
                    report.record(bytes);
                }
                Err(e) => eprintln!("warning: cache eviction: {e}"),
            }
        }
        report
    }

    /// Reports the cache directory's path, entry count, and byte total,
    /// plus the oldest and newest entry mtimes when any entry exists.
    pub fn info(&self) -> CacheInfo {
        self.stored().fold(
            CacheInfo {
                path: self.store.clone(),
                ..Default::default()
            },
            |mut acc, (_, m)| {
                acc.entries += 1;
                acc.bytes += on_disk(&m);
                if let Ok(t) = m.modified() {
                    acc.oldest_mtime = Some(acc.oldest_mtime.map_or(t, |o| o.min(t)));
                    acc.newest_mtime = acc.newest_mtime.max(Some(t));
                }
                acc
            },
        )
    }

    /// Atomically writes `value` for `key` via a temporary sidecar and
    /// `rename`. Any encode, write, or rename failure drops the insert
    /// silently and the tempfile is removed on drop. The size cap is
    /// honored by [`compact`](Self::compact). A landed write sets the
    /// flag [`inserted`](Self::inserted) reads.
    pub fn insert(&self, key: &CacheKey, value: &CacheEntryRef<'_>) {
        if self.try_insert(key, value).is_ok() {
            self.inserted.store(true, Ordering::Relaxed);
        }
    }

    /// True once a write has landed in this run.
    #[must_use]
    pub fn inserted(&self) -> bool {
        self.inserted.load(Ordering::Relaxed)
    }

    /// Returns the entry stored at `key` if present and well-formed,
    /// bumping the entry's mtime where the recorded one has aged past
    /// [`MTIME_GRANULARITY`].
    pub fn lookup(&self, key: &CacheKey) -> Option<CacheEntry> {
        let mut file = fs_err::File::open(self.path_for(key)).ok()?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).ok()?;
        let entry: CacheEntry = postcard::from_bytes(&bytes).ok()?;
        let now = SystemTime::now();
        if stale(&file, now) {
            let _ = file.set_modified(now);
        }
        Some(entry)
    }

    /// Opens or creates the cache directory with an unbounded size cap.
    ///
    /// Resolves the path through `PROSE_CACHE_DIR` →
    /// `dirs::cache_dir().join("prose")`.
    ///
    /// # Errors
    ///
    /// Returns `io::ErrorKind::NotFound` when no override is set and
    /// the platform exposes no cache directory, or any underlying IO
    /// error encountered while creating the directory.
    pub fn open() -> io::Result<Self> {
        let store = cache_root().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no platform cache directory is available",
            )
        })?;
        let root = store.join(generation());
        fs_err::create_dir_all(&root)?;
        Ok(Self::rooted(store, root))
    }

    /// True where `key` names bytes a write-back run of this generation
    /// landed as its own output, read from the ledger on first probe.
    pub fn owns_output(&self, key: &CacheKey) -> bool {
        self.own_output
            .get_or_init(|| {
                let Ok(bytes) = fs_err::read(self.own_output_path()) else {
                    return FxHashSet::default();
                };
                bytes.as_chunks::<32>().0.iter().copied().collect()
            })
            .contains(key.0.as_bytes())
    }

    pub(super) fn path_for(&self, key: &CacheKey) -> PathBuf {
        self.root.join(key.0.to_hex().as_str())
    }

    /// Marks `key`'s bytes as this run's own output, appended to the
    /// generation's ledger. An oversized ledger is truncated first,
    /// dropping its old markers.
    pub fn record_own_output(&self, key: &CacheKey) {
        let path = self.own_output_path();
        let oversized = fs_err::metadata(&path).is_ok_and(|meta| meta.len() > 1 << 20);
        let mut open = fs_err::OpenOptions::new();
        open.create(true);
        if oversized {
            open.write(true).truncate(true);
        } else {
            open.append(true);
        }
        if let Ok(mut file) = open.open(&path) {
            let _ = file.write_all(key.0.as_bytes());
        }
    }
}

fn cache_root() -> Option<PathBuf> {
    std::env::var_os("PROSE_CACHE_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::cache_dir().map(|d| d.join("prose")))
}

/// Sweeps the entries under `dir` into `report` and removes the
/// directory itself where the sweep left it empty.
fn sweep_dir(report: &mut CleanReport, dir: &Path) {
    report.absorb(sweep(dir));
    let _ = fs_err::remove_dir(dir);
}

/// The entries directly under `dir`, none where it cannot be read.
fn dir_entries(dir: &Path) -> impl Iterator<Item = DirEntry> + use<> {
    fs_err::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
}

/// The entry files directly under `dir`, paired with their metadata.
fn entries_in(dir: &Path) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
    dir_entries(dir)
        .filter(|e| is_entry_file(&e.path()))
        .filter_map(|e| e.metadata().ok().filter(Metadata::is_file).map(|m| (e, m)))
}

fn is_entry_file(path: &Path) -> bool {
    path.extension().is_none() && path.file_name().is_none_or(|name| name != "own-output")
}

/// The level `cap` is driven down to once an eviction pass starts.
fn low_water(cap: u64) -> u64 {
    cap / LOW_WATER_DIVISOR * LOW_WATER_MULTIPLIER
}

/// The space one entry occupies, in the whole blocks the filesystem
/// allocated to it.
#[cfg(unix)]
fn on_disk(metadata: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    metadata.blocks() * 512
}

#[cfg(not(unix))]
fn on_disk(metadata: &Metadata) -> u64 {
    metadata.len().next_multiple_of(4096)
}

/// True where `file`'s recorded mtime is older than
/// [`MTIME_GRANULARITY`], or where the metadata read fails.
fn stale(file: &fs_err::File, now: SystemTime) -> bool {
    file.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|m| now.duration_since(m).ok())
        .is_none_or(|age| age >= MTIME_GRANULARITY)
}

/// Removes every file directly under `dir`, reporting what went.
fn sweep(dir: &Path) -> CleanReport {
    let mut report = CleanReport::default();
    for entry in dir_entries(dir) {
        remove_into(&mut report, &entry);
    }
    report
}

/// Removes `entry`'s file, recording what it freed.
fn remove_into(report: &mut CleanReport, entry: &DirEntry) {
    let bytes = entry.metadata().as_ref().map_or(0, on_disk);
    if fs_err::remove_file(entry.path()).is_ok() {
        report.record(bytes);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        cache::{
            NotebookCells, NotebookCellsRef, Rewrite,
            key::tests::{CONFIG_A, key, rules},
        },
        diagnostics::Diagnostic,
        rules::align_equals::AlignEquals,
        testing::{format_diagnostic, range},
        unstable::UnstableRewrite,
    };

    /// Backdates the file or directory at `path` past the sweep's grace
    /// window.
    fn age(path: &Path) {
        let stale = SystemTime::now() - Duration::from_hours(24);
        fs_err::File::open(path)
            .expect("opens the path")
            .set_modified(stale)
            .expect("backdates the path");
    }

    fn cache_in(tmp: &TempDir, max_mib: u32) -> Cache {
        Cache {
            max_size_bytes: u64::from(max_mib) * 1024 * 1024,
            ..Cache::in_store(tmp.path().join("cache"))
        }
    }

    /// Builds a fresh temp dir beside a 100 MiB cache rooted inside it.
    fn cached() -> (TempDir, Cache) {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        (tmp, cache)
    }

    fn entry(formatted: &str) -> CacheEntry {
        CacheEntry {
            // The rule must be a registered slug, because `RuleId`
            // deserializes through the registry and an unknown slug
            // fails the entry's round-trip.
            diagnostics: vec![Diagnostic {
                rule: AlignEquals::SLUG,
                ..format_diagnostic(range(0, 1))
            }],
            notebook: None,
            rewrite: Rewrite::text(formatted.to_owned()),
            unstable: None,
        }
    }

    /// Writes a generation directory beside the cache's own, holding one
    /// entry-shaped file.
    fn generation_beside(cache: &Cache, name: &str) -> std::path::PathBuf {
        let dir = cache.store.join(name);
        fs_err::create_dir_all(&dir).expect("creates");
        fs_err::write(dir.join("cafebabe"), b"an earlier build's entry").expect("writes");
        dir
    }

    /// Writes `entry` under `key` the way the run path does, borrowing
    /// the diagnostics and rewrite.
    fn insert(cache: &Cache, key: &CacheKey, entry: &CacheEntry) {
        cache.insert(
            key,
            &CacheEntryRef {
                diagnostics: &entry.diagnostics,
                notebook: entry.notebook.as_ref().map(|cells| NotebookCellsRef {
                    code: &cells.code,
                    index: cells.index.as_ref(),
                }),
                rewrite: &entry.rewrite,
                unstable: entry.unstable.as_deref(),
            },
        );
    }

    #[test]
    fn clean_clears_every_entry_and_returns_report() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        insert(&cache, &key, &entry("y = 1\n"));
        let report = cache.clean().expect("cleans");
        assert_eq!(report.entries, 1);
        assert!(report.bytes > 0);
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn clean_returns_zeros_on_empty_cache() {
        let (_tmp, cache) = cached();
        assert_eq!(cache.clean().expect("cleans"), CleanReport::default());
    }

    #[test]
    fn compact_evicts_below_the_low_water_mark() {
        let (_tmp, cache) = cached();
        for i in 0..10 {
            insert(
                &cache,
                &key(format!("x = {i}\n").as_bytes(), CONFIG_A, rules()),
                &entry("a = 1\n"),
            );
        }

        let capped = Cache {
            max_entries: 5,
            ..cache
        };
        capped.compact();

        // A pass that stopped at the cap would leave five, putting the
        // next insert back over it.
        assert_eq!(capped.info().entries, 4);
    }

    #[test]
    fn compact_evicts_until_under_cap() {
        let (_tmp, cache) = cached();
        insert(
            &cache,
            &key(b"x = 1\n", CONFIG_A, rules()),
            &entry("a = 1\n"),
        );
        insert(
            &cache,
            &key(b"y = 2\n", CONFIG_A, rules()),
            &entry("b = 2\n"),
        );

        let tightened = Cache {
            max_size_bytes: 0,
            ..cache
        };
        let report = tightened.compact();

        assert_eq!(report.entries, 2);
        assert!(report.bytes > 0);
        assert_eq!(tightened.info().entries, 0);
    }

    #[test]
    fn compact_holds_a_generation_touched_inside_the_grace_window() {
        let (_tmp, cache) = cached();
        let live = generation_beside(&cache, "live");

        let report = cache.compact();

        assert_eq!(report.entries, 0);
        assert!(live.exists());
    }

    #[test]
    fn compact_holds_one_entry_at_the_smallest_cap() {
        let (_tmp, cache) = cached();
        let key_old = key(b"x = 1\n", CONFIG_A, rules());
        let key_new = key(b"y = 2\n", CONFIG_A, rules());
        insert(&cache, &key_old, &entry("a = 1\n"));
        insert(&cache, &key_new, &entry("b = 2\n"));
        age(&cache.path_for(&key_old));

        let capped = Cache {
            max_entries: 1,
            ..cache
        };
        let report = capped.compact();

        assert_eq!(report.entries, 1);
        assert!(capped.lookup(&key_old).is_none());
        assert!(capped.lookup(&key_new).is_some());
    }

    #[test]
    fn compact_prunes_a_generation_past_the_grace_window() {
        let (_tmp, cache) = cached();
        let dead = generation_beside(&cache, "dead");
        age(&dead);

        let report = cache.compact();

        assert_eq!(report.entries, 1);
        assert!(!dead.exists());
    }

    #[test]
    fn compact_removes_an_entry_left_directly_in_the_store() {
        let (_tmp, cache) = cached();
        let flat = cache.store.join("deadbeef");
        fs_err::write(&flat, b"a pre-generation entry").expect("writes");

        let report = cache.compact();

        assert_eq!(report.entries, 1);
        assert!(!flat.exists());
    }

    #[test]
    fn compact_returns_zeros_when_under_cap() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        insert(&cache, &key, &entry("y = 1\n"));
        assert_eq!(cache.compact(), CleanReport::default());
    }

    #[test]
    fn info_and_compact_leave_the_own_output_ledger_alone() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        cache.record_own_output(&key);

        assert_eq!(cache.info().entries, 0);
        assert_eq!(cache.compact(), CleanReport::default());
        assert!(cache.owns_output(&key));
    }

    #[test]
    fn info_counts_entries_across_generations() {
        let (_tmp, cache) = cached();
        insert(
            &cache,
            &key(b"x = 1\n", CONFIG_A, rules()),
            &entry("a = 1\n"),
        );
        generation_beside(&cache, "older");

        assert_eq!(cache.info().entries, 2);
    }

    #[test]
    fn info_counts_entries_and_byte_total() {
        let (_tmp, cache) = cached();
        insert(
            &cache,
            &key(b"x = 1\n", CONFIG_A, rules()),
            &entry("y = 1\n"),
        );
        insert(
            &cache,
            &key(b"x = 2\n", CONFIG_A, rules()),
            &entry("y = 2\n"),
        );
        let info = cache.info();
        assert_eq!(info.entries, 2);
        assert!(info.bytes > 0);
        assert!(info.oldest_mtime.is_some());
        assert!(info.newest_mtime.is_some());
    }

    #[test]
    fn info_reports_zeros_on_empty_cache() {
        let (_tmp, cache) = cached();
        let info = cache.info();
        assert_eq!(info.entries, 0);
        assert_eq!(info.bytes, 0);
        assert!(info.oldest_mtime.is_none());
        assert!(info.newest_mtime.is_none());
    }

    #[test]
    fn info_skips_tmp_sidecars() {
        let (_tmp, cache) = cached();
        fs_err::write(cache.root.join("orphan.123.tmp"), b"in flight").expect("writes");
        let info = cache.info();
        assert_eq!(info.entries, 0);
        assert_eq!(info.bytes, 0);
    }

    #[test]
    fn insert_holds_an_over_cap_entry_until_compact_runs() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 0);
        let key = key(b"x = 1\n", CONFIG_A, rules());

        insert(&cache, &key, &entry("y = 1\n"));

        assert!(cache.lookup(&key).is_some());
        cache.compact();
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn insert_leaves_no_tmp_sidecar_on_success() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        insert(&cache, &key, &entry("y = 1\n"));
        let tmp_count = fs_err::read_dir(&cache.root)
            .expect("read_dir")
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "tmp"))
            .count();
        assert_eq!(tmp_count, 0);
    }

    #[rstest]
    #[case::the_entry(entry("y = 1\n"))]
    #[case::a_notebook_rewrite(CacheEntry {
        diagnostics: Vec::new(),
        notebook: Some(NotebookCells {
            code: "x = 1\n".to_owned(),
            index: None,
        }),
        rewrite: Rewrite::notebook(
            vec!["x = 1\n".to_owned()],
            vec!["x  = 1\n".to_owned()],
            "{}\n".to_owned(),
        ),
        unstable: None,
    })]
    #[case::a_settle_report(CacheEntry {
        unstable: Some(Box::new(UnstableRewrite {
            config_toml: CONFIG_A.to_owned(),
            first: "yy = 1\n".to_owned(),
            rules: vec![AlignEquals::SLUG],
            second: "yyy = 1\n".to_owned(),
        })),
        ..entry("yy = 1\n")
    })]
    #[case::a_skipped_rewrite(CacheEntry {
        diagnostics: Vec::new(),
        notebook: None,
        rewrite: Rewrite::Skipped,
        unstable: None,
    })]
    fn insert_then_lookup_round_trips(#[case] original: CacheEntry) {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        insert(&cache, &key, &original);
        assert_eq!(cache.lookup(&key).expect("hit"), original);
    }

    #[rstest]
    #[case::a_corrupt_entry(Some(b"not postcard bytes".as_slice()))]
    #[case::a_missing_entry(None)]
    fn lookup_returns_none_for(#[case] written: Option<&[u8]>) {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        if let Some(bytes) = written {
            fs_err::write(cache.path_for(&key), bytes).expect("writes");
        }
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn own_output_marker_round_trips_across_instances() {
        let (tmp, cache) = cached();
        let marked = key(b"y = 1\n", CONFIG_A, rules());
        let unmarked = key(b"z = 2\n", CONFIG_A, rules());
        cache.record_own_output(&marked);

        let reopened = Cache {
            inserted: std::sync::atomic::AtomicBool::new(false),
            ..cache_in(&tmp, 100)
        };

        assert!(reopened.owns_output(&marked));
        assert!(!reopened.owns_output(&unmarked));
    }
}

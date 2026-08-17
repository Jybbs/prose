//! The on-disk cache store: directory walk, atomic insert, and
//! LRU eviction.

use std::{
    fs::Metadata,
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use fs_err::DirEntry;
use tempfile::NamedTempFile;

use super::{CacheEntry, CacheEntryRef, CacheInfo, CacheKey, CleanReport, key::generation};

/// User-level on-disk cache.
#[derive(Debug)]
pub struct Cache {
    pub(super) max_entries: usize,
    pub(super) max_size_bytes: u64,
    /// This build's generation directory, where every entry lands.
    pub(super) root: PathBuf,
    /// The directory holding every generation, swept for dead ones.
    pub(super) store: PathBuf,
}

impl Cache {
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
        Ok(Self {
            max_entries: usize::MAX,
            max_size_bytes: u64::MAX,
            root,
            store,
        })
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

    fn entries(&self) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
        entries_in(&self.root)
    }

    /// Every entry the store holds, across this build's generation and
    /// any older one a sweep has not yet reclaimed.
    fn stored(&self) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
        let generations: Vec<PathBuf> = fs_err::read_dir(&self.store)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path())
            .collect();
        entries_in(&self.store).chain(generations.into_iter().flat_map(|d| entries_in(&d)))
    }

    pub(super) fn path_for(&self, key: &CacheKey) -> PathBuf {
        self.root.join(key.0.to_hex().as_str())
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
                report.absorb(sweep(&path));
                let _ = fs_err::remove_dir(&path);
                continue;
            }
            let bytes = entry.metadata().map_or(0, |m| m.len());
            if fs_err::remove_file(&path).is_ok() {
                report.record(bytes);
            }
        }
        fs_err::create_dir_all(&self.root)?;
        Ok(report)
    }

    /// Runs the LRU eviction pass to honor the configured size and
    /// entry-count caps, and returns what it removed. Each entry counts
    /// the space the filesystem allocated to it rather than its own
    /// length, so the total tracks what the directory costs on disk.
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

    /// True where a directory of `count` entries occupying `total` bytes
    /// sits inside both configured caps.
    fn within_caps(&self, total: u64, count: usize) -> bool {
        total <= self.max_size_bytes && count <= self.max_entries
    }

    /// True where an eviction pass has driven the directory far enough
    /// below its caps to stop. Stopping at the cap itself leaves the
    /// next insert over it again, so every run pays a full walk and a
    /// removal, whereas stopping below buys many runs of headroom for
    /// one pass. The entry floor holds at one, so a cap of one still
    /// keeps an entry.
    fn below_low_water(&self, total: u64, count: usize) -> bool {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.max_entries).unwrap_or(u64::MAX);
        total <= low_water(self.max_size_bytes) && count <= low_water(entries).max(1)
    }

    /// Removes every generation directory but this build's, plus any
    /// entry file left directly in the store by a build that predates
    /// the generation layout. A directory touched within
    /// [`GENERATION_GRACE`] is left alone, so a concurrently running
    /// build of another version keeps its own entries.
    fn prune_dead_generations(&self) -> CleanReport {
        let mut report = CleanReport::default();
        let now = SystemTime::now();
        for entry in fs_err::read_dir(&self.store)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
        {
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
            report.absorb(sweep(&path));
            let _ = fs_err::remove_dir(&path);
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
                acc.bytes += m.len();
                if let Ok(t) = m.modified() {
                    acc.oldest_mtime = Some(acc.oldest_mtime.map_or(t, |o| o.min(t)));
                    acc.newest_mtime = Some(acc.newest_mtime.map_or(t, |n| n.max(t)));
                }
                acc
            },
        )
    }

    /// Atomically writes `value` for `key` via a temporary sidecar and
    /// `rename`. Any encode, write, or rename failure drops the insert
    /// silently and lets the tempfile clean itself up on drop. The size
    /// cap is honored by [`compact`](Self::compact).
    pub fn insert(&self, key: &CacheKey, value: &CacheEntryRef<'_>) {
        let _ = self.try_insert(key, value);
    }

    /// Returns the entry stored at `key` if present and well-formed,
    /// bumping the entry's mtime where the recorded one has aged past
    /// [`MTIME_GRANULARITY`]. Eviction orders by mtime at day
    /// granularity at most, so a bump per hit would write an inode on
    /// every read of an edit loop to sharpen an order nothing reads
    /// that finely.
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
}

/// How far an entry's recorded mtime may lag the clock before a hit
/// rewrites it.
const MTIME_GRANULARITY: Duration = Duration::from_secs(60 * 60);

/// How long a generation directory must sit untouched before a sweep
/// reclaims it, leaving a concurrently running build of another
/// version its own entries.
const GENERATION_GRACE: Duration = Duration::from_secs(60 * 60);

/// The fraction of each cap an eviction pass drives down to, as a
/// multiplier over a divisor so the arithmetic stays in integers.
const LOW_WATER_DIVISOR: u64 = 5;
const LOW_WATER_MULTIPLIER: u64 = 4;

/// The level `cap` is driven down to once an eviction pass starts.
fn low_water(cap: u64) -> u64 {
    cap / LOW_WATER_DIVISOR * LOW_WATER_MULTIPLIER
}

fn cache_root() -> Option<PathBuf> {
    std::env::var_os("PROSE_CACHE_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::cache_dir().map(|d| d.join("prose")))
}

/// The space one entry occupies, which a filesystem allocates in whole
/// blocks rather than in the entry's own length.
#[cfg(unix)]
fn on_disk(metadata: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    metadata.blocks() * 512
}

#[cfg(not(unix))]
fn on_disk(metadata: &Metadata) -> u64 {
    metadata.len().next_multiple_of(4096)
}

fn is_entry_file(path: &Path) -> bool {
    path.extension().is_none()
}

/// The entry files directly under `dir`, paired with their metadata.
fn entries_in(dir: &Path) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
    fs_err::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| is_entry_file(&e.path()))
        .filter_map(|e| e.metadata().ok().filter(Metadata::is_file).map(|m| (e, m)))
}

/// Removes every file directly under `dir`, reporting what went.
fn sweep(dir: &Path) -> CleanReport {
    let mut report = CleanReport::default();
    for entry in fs_err::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        let bytes = entry.metadata().map_or(0, |m| m.len());
        if fs_err::remove_file(entry.path()).is_ok() {
            report.record(bytes);
        }
    }
    report
}

/// True where `file`'s recorded mtime is older than
/// [`MTIME_GRANULARITY`], or where the metadata read fails, since a
/// file whose age cannot be read is bumped rather than left to sort
/// as the oldest entry in the directory.
fn stale(file: &fs_err::File, now: SystemTime) -> bool {
    file.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|m| now.duration_since(m).ok())
        .is_none_or(|age| age >= MTIME_GRANULARITY)
}

//! The on-disk cache store: directory walk, atomic insert, and
//! LRU eviction.

use std::{
    fs::Metadata,
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use bincode::{
    config::standard,
    serde::{decode_from_std_read, encode_into_std_write},
};
use fs_err::DirEntry;
use tempfile::NamedTempFile;

use super::{CacheEntry, CacheInfo, CacheKey, CleanReport};

/// User-level on-disk cache.
#[derive(Debug)]
pub struct Cache {
    pub(super) max_size_bytes: u64,
    pub(super) root: PathBuf,
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
        let root = cache_root().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no platform cache directory is available",
            )
        })?;
        fs_err::create_dir_all(&root)?;
        Ok(Self {
            max_size_bytes: u64::MAX,
            root,
        })
    }

    /// Sets the LRU eviction cap in MiB.
    #[must_use]
    pub fn with_max_size_mib(mut self, mib: u32) -> Self {
        self.max_size_bytes = u64::from(mib) * 1024 * 1024;
        self
    }

    fn entries(&self) -> impl Iterator<Item = (DirEntry, Metadata)> + use<> {
        fs_err::read_dir(&self.root)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|e| is_entry_file(&e.path()))
            .filter_map(|e| e.metadata().ok().map(|m| (e, m)))
    }

    pub(super) fn path_for(&self, key: &CacheKey) -> PathBuf {
        self.root.join(key.0.to_hex().as_str())
    }

    fn try_insert(&self, key: &CacheKey, value: &CacheEntry) -> io::Result<()> {
        let mut tmp = NamedTempFile::with_suffix_in(".tmp", &self.root)?;
        {
            let mut buf = BufWriter::new(tmp.as_file_mut());
            encode_into_std_write(value, &mut buf, standard()).map_err(io::Error::other)?;
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
        for entry in fs_err::read_dir(&self.root)? {
            let entry = entry?;
            let bytes = entry.metadata().map_or(0, |m| m.len());
            if fs_err::remove_file(entry.path()).is_ok() {
                report.bytes += bytes;
                report.entries += 1;
            }
        }
        Ok(report)
    }

    /// Runs the LRU eviction pass to honor the configured size cap and
    /// returns what it removed.
    pub fn compact(&self) -> CleanReport {
        let mut report = CleanReport::default();
        let mut files: Vec<(SystemTime, u64, DirEntry)> = self
            .entries()
            .map(|(e, m)| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len(), e))
            .collect();
        let mut total: u64 = files.iter().map(|(_, bytes, _)| *bytes).sum();
        if total <= self.max_size_bytes {
            return report;
        }
        files.sort_by_key(|(mtime, _, _)| *mtime);
        for (_, bytes, entry) in files {
            if total <= self.max_size_bytes {
                break;
            }
            match fs_err::remove_file(entry.path()) {
                Ok(()) => {
                    total = total.saturating_sub(bytes);
                    report.bytes += bytes;
                    report.entries += 1;
                }
                Err(e) => eprintln!("warning: cache eviction: {e}"),
            }
        }
        report
    }

    /// Reports the cache directory's path, entry count, and byte total,
    /// plus the oldest and newest entry mtimes when any entry exists.
    pub fn info(&self) -> CacheInfo {
        self.entries().fold(
            CacheInfo {
                path: self.root.clone(),
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
    /// `rename`, then runs best-effort LRU eviction. Any encode, write,
    /// or rename failure drops the insert silently and lets the
    /// tempfile clean itself up on drop.
    pub fn insert(&self, key: &CacheKey, value: &CacheEntry) {
        let _ = self.try_insert(key, value);
        self.compact();
    }

    /// Returns the entry stored at `key` if present and well-formed,
    /// bumping the entry's mtime.
    pub fn lookup(&self, key: &CacheKey) -> Option<CacheEntry> {
        let file = fs_err::File::open(self.path_for(key)).ok()?;
        let entry: CacheEntry =
            decode_from_std_read(&mut BufReader::new(&file), standard()).ok()?;
        let _ = file.set_modified(SystemTime::now());
        Some(entry)
    }
}

fn cache_root() -> Option<PathBuf> {
    std::env::var_os("PROSE_CACHE_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::cache_dir().map(|d| d.join("prose")))
}

fn is_entry_file(path: &Path) -> bool {
    path.extension().is_none()
}

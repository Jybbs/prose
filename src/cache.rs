//! User-level content-addressed cache for `prose check` and `prose format`.
//!
//! Keys are BLAKE3 digests over the source bytes, the canonical TOML
//! serialization of the active `Config`, the Prose version, and a
//! private `CACHE_FORMAT_VERSION` that bumps independently when the
//! on-disk entry shape changes. Entries live one file per key under
//! the platform's cache directory, with the path resolving through
//! `PROSE_CACHE_DIR` → `dirs::cache_dir()`. Inserts write to a
//! temporary sibling then `rename` onto the final path, so a
//! concurrent reader never observes a partial entry. LRU eviction by
//! mtime caps the directory at the configured size on every insert.

use std::fs::Metadata;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bincode::config::standard;
use bincode::serde::{decode_from_std_read, encode_into_std_write};
use fs_err::DirEntry;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::diagnostics::Diagnostic;

const CACHE_FORMAT_VERSION: &str = "1";

/// User-level on-disk cache.
#[derive(Debug)]
pub struct Cache {
    max_size_bytes: u64,
    root: PathBuf,
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

    fn path_for(&self, key: &CacheKey) -> PathBuf {
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

/// Post-pipeline state cached per `(source, config, version)` key.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CacheEntry {
    pub diagnostics: Vec<Diagnostic>,
    pub formatted_source: Option<String>,
}

/// Snapshot of the cache directory's contents at one point in time.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CacheInfo {
    pub bytes: u64,
    pub entries: usize,
    pub newest_mtime: Option<SystemTime>,
    pub oldest_mtime: Option<SystemTime>,
    pub path: PathBuf,
}

/// BLAKE3 digest of `(source_bytes ++ config_toml ++ prose_version ++ cache_format_version)`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(blake3::Hash);

impl CacheKey {
    /// Computes the key for a source file under the pre-serialized config TOML.
    pub fn compute(source_bytes: &[u8], config_toml: &str) -> Self {
        Self::compute_with_versions(
            source_bytes,
            config_toml,
            env!("CARGO_PKG_VERSION"),
            CACHE_FORMAT_VERSION,
        )
    }

    fn compute_with_versions(
        source_bytes: &[u8],
        config_toml: &str,
        prose_version: &str,
        format_version: &str,
    ) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(source_bytes);
        hasher.update(config_toml.as_bytes());
        hasher.update(prose_version.as_bytes());
        hasher.update(format_version.as_bytes());
        Self(hasher.finalize())
    }
}

/// Outcome of a `Cache::clean` or `Cache::compact` call.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CleanReport {
    pub bytes: u64,
    pub entries: usize,
}

fn cache_root() -> Option<PathBuf> {
    std::env::var_os("PROSE_CACHE_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::cache_dir().map(|d| d.join("prose")))
}

fn is_entry_file(path: &Path) -> bool {
    path.extension().is_none()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;
    use tempfile::TempDir;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::RuleId;

    const CONFIG_A: &str = "code-line-length = 88\n";
    const CONFIG_B: &str = "code-line-length = 100\n";

    fn cache_in(tmp: &TempDir, max_mib: u32) -> Cache {
        let root = tmp.path().join("cache");
        std::fs::create_dir_all(&root).expect("creates");
        Cache {
            max_size_bytes: u64::from(max_mib) * 1024 * 1024,
            root,
        }
    }

    fn entry(formatted: Option<&str>) -> CacheEntry {
        CacheEntry {
            diagnostics: vec![Diagnostic {
                fix: Some(vec![Edit::range_replacement("y".into(), range(0, 1))]),
                message: "rewrite".into(),
                range: range(0, 1),
                rule: RuleId::from("align-equals"),
                severity: Severity::Format,
            }],
            formatted_source: formatted.map(str::to_owned),
        }
    }

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn cache_key_differs_when_cache_format_version_changes() {
        let key_a =
            CacheKey::compute_with_versions(b"x = 1\n", CONFIG_A, env!("CARGO_PKG_VERSION"), "1");
        let key_b =
            CacheKey::compute_with_versions(b"x = 1\n", CONFIG_A, env!("CARGO_PKG_VERSION"), "2");
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_differs_when_config_changes() {
        let key_a = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let key_b = CacheKey::compute(b"x = 1\n", CONFIG_B);
        assert_ne!(key_a, key_b);
        let key_c = CacheKey::compute(b"x = 1\n", CONFIG_B);
        assert_eq!(key_b, key_c);
    }

    #[test]
    fn cache_key_differs_when_prose_version_changes() {
        let key_a =
            CacheKey::compute_with_versions(b"x = 1\n", CONFIG_A, "0.2.3", CACHE_FORMAT_VERSION);
        let key_b =
            CacheKey::compute_with_versions(b"x = 1\n", CONFIG_A, "0.3.0", CACHE_FORMAT_VERSION);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_differs_when_source_changes() {
        let key_a = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let key_b = CacheKey::compute(b"x = 2\n", CONFIG_A);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_hex_renders_64_lowercase_chars() {
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let hex = key.0.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn cache_key_is_stable_across_runs() {
        assert_eq!(
            CacheKey::compute(b"x = 1\n", CONFIG_A),
            CacheKey::compute(b"x = 1\n", CONFIG_A),
        );
    }

    #[test]
    fn clean_clears_every_entry_and_returns_report() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        cache.insert(&key, &entry(Some("y = 1\n")));
        let report = cache.clean().expect("cleans");
        assert_eq!(report.entries, 1);
        assert!(report.bytes > 0);
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn clean_returns_zeros_on_empty_cache() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let report = cache.clean().expect("cleans");
        assert_eq!(report.entries, 0);
        assert_eq!(report.bytes, 0);
    }

    #[test]
    fn compact_evicts_until_under_cap() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key_old = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let key_new = CacheKey::compute(b"y = 2\n", CONFIG_A);
        cache.insert(&key_old, &entry(Some("a = 1\n")));
        std::thread::sleep(std::time::Duration::from_millis(20));
        cache.insert(&key_new, &entry(Some("b = 2\n")));

        let tightened = Cache {
            max_size_bytes: 0,
            root: cache.root.clone(),
        };
        let report = tightened.compact();

        assert!(report.entries >= 1);
        assert!(report.bytes > 0);
    }

    #[test]
    fn compact_returns_zeros_when_under_cap() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        cache.insert(&key, &entry(Some("y = 1\n")));
        let report = cache.compact();
        assert_eq!(report.entries, 0);
        assert_eq!(report.bytes, 0);
    }

    #[test]
    fn info_counts_entries_and_byte_total() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        cache.insert(
            &CacheKey::compute(b"x = 1\n", CONFIG_A),
            &entry(Some("y = 1\n")),
        );
        cache.insert(
            &CacheKey::compute(b"x = 2\n", CONFIG_A),
            &entry(Some("y = 2\n")),
        );
        let info = cache.info();
        assert_eq!(info.entries, 2);
        assert!(info.bytes > 0);
        assert!(info.oldest_mtime.is_some());
        assert!(info.newest_mtime.is_some());
    }

    #[test]
    fn info_reports_zeros_on_empty_cache() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let info = cache.info();
        assert_eq!(info.entries, 0);
        assert_eq!(info.bytes, 0);
        assert!(info.oldest_mtime.is_none());
        assert!(info.newest_mtime.is_none());
    }

    #[test]
    fn info_skips_tmp_sidecars() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        fs_err::write(cache.root.join("orphan.123.tmp"), b"in flight").expect("writes");
        let info = cache.info();
        assert_eq!(info.entries, 0);
        assert_eq!(info.bytes, 0);
    }

    #[test]
    fn insert_evicts_oldest_when_above_cap() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 0);
        let key_old = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let key_new = CacheKey::compute(b"y = 2\n", CONFIG_A);
        cache.insert(&key_old, &entry(Some("a = 1\n")));
        std::thread::sleep(std::time::Duration::from_millis(20));
        cache.insert(&key_new, &entry(Some("b = 2\n")));
        assert!(cache.lookup(&key_old).is_none());
    }

    #[test]
    fn insert_leaves_no_tmp_sidecar_on_success() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        cache.insert(&key, &entry(Some("y = 1\n")));
        let tmp_count = fs_err::read_dir(&cache.root)
            .expect("read_dir")
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "tmp"))
            .count();
        assert_eq!(tmp_count, 0);
    }

    #[test]
    fn insert_then_lookup_round_trips_the_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let original = entry(Some("y = 1\n"));
        cache.insert(&key, &original);
        let recovered = cache.lookup(&key).expect("hit");
        assert_eq!(recovered, original);
    }

    #[test]
    fn lookup_returns_none_for_corrupt_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        fs_err::write(cache.path_for(&key), b"not bincode bytes").expect("writes");
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn lookup_returns_none_for_missing_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        assert!(cache.lookup(&key).is_none());
    }
}

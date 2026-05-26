//! User-level content-addressed cache for `prose check` and `prose format`.
//!
//! Keys are BLAKE3 digests over the source bytes, the canonical TOML
//! serialization of the active `Config`, and the Prose version.
//! Entries live one file per key under the platform's cache directory
//! (`~/Library/Caches/prose` on macOS, `$XDG_CACHE_HOME/prose` on Linux,
//! `%LOCALAPPDATA%\prose\cache` on Windows). LRU eviction by mtime caps
//! the directory at the configured size on every insert.

use std::path::PathBuf;
use std::time::SystemTime;

use bincode::config::standard as bincode_config;
use bincode::serde::{decode_from_slice, encode_to_vec};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::diagnostics::Diagnostic;

const SUBDIR: &str = "prose";

/// User-level on-disk cache.
#[derive(Debug)]
pub struct Cache {
    max_size_bytes: u64,
    root: PathBuf,
}

impl Cache {
    /// Opens or creates the cache directory with an unbounded size cap.
    ///
    /// # Errors
    ///
    /// Returns `CacheError::NoCacheDir` when the platform exposes no
    /// cache directory, and `CacheError::Io` when the directory cannot
    /// be created.
    pub fn open() -> Result<Self, CacheError> {
        let root = dirs::cache_dir()
            .ok_or(CacheError::NoCacheDir)?
            .join(SUBDIR);
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

    fn evict(&self) {
        let Ok(entries) = fs_err::read_dir(&self.root) else {
            return;
        };
        let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
            .filter_map(|e| {
                let e = e.ok()?;
                let m = e.metadata().ok()?;
                let mtime = m.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                Some((mtime, m.len(), e.path()))
            })
            .collect();
        let mut total: u64 = files.iter().map(|(_, bytes, _)| *bytes).sum();
        if total <= self.max_size_bytes {
            return;
        }
        files.sort_by_key(|(mtime, _, _)| *mtime);
        for (_, bytes, path) in files {
            if total <= self.max_size_bytes {
                break;
            }
            if fs_err::remove_file(&path).is_ok() {
                total = total.saturating_sub(bytes);
            } else {
                eprintln!(
                    "warning: cache eviction could not remove {}",
                    path.display()
                );
            }
        }
    }

    fn path_for(&self, key: &CacheKey) -> PathBuf {
        self.root.join(key.hex())
    }

    /// Removes every cache file and returns the count and freed bytes.
    ///
    /// # Errors
    ///
    /// Returns `CacheError::Io` if the cache directory cannot be read.
    pub fn clean(&self) -> Result<CleanReport, CacheError> {
        let mut report = CleanReport::default();
        for entry in fs_err::read_dir(&self.root)? {
            let entry = entry?;
            let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if fs_err::remove_file(entry.path()).is_ok() {
                report.bytes += bytes;
                report.entries += 1;
            }
        }
        Ok(report)
    }

    /// Writes `value` for `key`, then runs best-effort LRU eviction.
    pub fn insert(&self, key: &CacheKey, value: &CacheEntry) {
        let Ok(bytes) = encode_to_vec(value, bincode_config()) else {
            return;
        };
        if fs_err::write(self.path_for(key), bytes).is_ok() {
            self.evict();
        }
    }

    /// Returns the entry stored at `key` if present and well-formed.
    pub fn lookup(&self, key: &CacheKey) -> Option<CacheEntry> {
        let path = self.path_for(key);
        let bytes = fs_err::read(&path).ok()?;
        let (entry, _): (CacheEntry, _) = decode_from_slice(&bytes, bincode_config()).ok()?;
        if let Ok(file) = std::fs::File::open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
        Some(entry)
    }
}

/// Post-pipeline state cached per `(source, config, version)` key.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CacheEntry {
    pub diagnostics: Vec<Diagnostic>,
    pub formatted_source: Option<String>,
}

/// Failure modes surfaced when opening or cleaning the cache.
#[derive(Debug, Error)]
pub enum CacheError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("no platform cache directory is available")]
    NoCacheDir,
}

/// BLAKE3 digest of `(source_bytes ++ config_toml ++ prose_version)`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(blake3::Hash);

impl CacheKey {
    /// Computes the key for a source file under the pre-serialized config TOML.
    pub fn compute(source_bytes: &[u8], config_toml: &str) -> Self {
        Self::compute_with_version(source_bytes, config_toml, env!("CARGO_PKG_VERSION"))
    }

    fn compute_with_version(source_bytes: &[u8], config_toml: &str, version: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(source_bytes);
        hasher.update(config_toml.as_bytes());
        hasher.update(version.as_bytes());
        Self(hasher.finalize())
    }

    /// Returns the 64-character lowercase hex form of the digest.
    pub fn hex(&self) -> String {
        self.0.to_hex().to_string()
    }
}

/// Outcome of a `Cache::clean` call.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CleanReport {
    pub bytes: u64,
    pub entries: usize,
}

#[cfg(test)]
mod tests {
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
                fix: Some(Edit::range_replacement("y".into(), range(0, 1))),
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
    fn cache_key_differs_when_config_changes() {
        let key_a = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let key_b = CacheKey::compute(b"x = 1\n", CONFIG_B);
        assert_ne!(key_a, key_b);
        let key_c = CacheKey::compute(b"x = 1\n", CONFIG_B);
        assert_eq!(key_b, key_c);
    }

    #[test]
    fn cache_key_differs_when_prose_version_changes() {
        let key_a = CacheKey::compute_with_version(b"x = 1\n", CONFIG_A, "0.2.3");
        let key_b = CacheKey::compute_with_version(b"x = 1\n", CONFIG_A, "0.3.0");
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
        let hex = key.hex();
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
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
    fn evict_drops_oldest_entries_when_above_cap() {
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

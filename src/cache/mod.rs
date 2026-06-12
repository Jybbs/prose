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

mod engine;
mod key;
mod records;

pub use engine::Cache;
pub use key::CacheKey;
pub use records::{CacheEntry, CacheInfo, CleanReport, Rewrite};

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::key::CACHE_FORMAT_VERSION;
    use super::*;
    use crate::diagnostics::Diagnostic;
    use crate::rule::RuleId;
    use crate::testing::{format_diagnostic, range};

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

    fn entry(formatted: &str) -> CacheEntry {
        CacheEntry {
            // The rule must be a registered slug, because `RuleId`
            // deserializes through the registry and an unknown slug
            // fails the entry's round-trip.
            diagnostics: vec![Diagnostic {
                rule: RuleId::from("align-equals"),
                ..format_diagnostic(range(0, 1))
            }],
            rewrite: Rewrite::Changed(formatted.to_owned()),
        }
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
        cache.insert(&key, &entry("y = 1\n"));
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
        cache.insert(&key_old, &entry("a = 1\n"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        cache.insert(&key_new, &entry("b = 2\n"));

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
        cache.insert(&key, &entry("y = 1\n"));
        let report = cache.compact();
        assert_eq!(report.entries, 0);
        assert_eq!(report.bytes, 0);
    }

    #[test]
    fn info_counts_entries_and_byte_total() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        cache.insert(&CacheKey::compute(b"x = 1\n", CONFIG_A), &entry("y = 1\n"));
        cache.insert(&CacheKey::compute(b"x = 2\n", CONFIG_A), &entry("y = 2\n"));
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
        cache.insert(&key_old, &entry("a = 1\n"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        cache.insert(&key_new, &entry("b = 2\n"));
        assert!(cache.lookup(&key_old).is_none());
    }

    #[test]
    fn insert_leaves_no_tmp_sidecar_on_success() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        cache.insert(&key, &entry("y = 1\n"));
        let tmp_count = fs_err::read_dir(&cache.root)
            .expect("read_dir")
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "tmp"))
            .count();
        assert_eq!(tmp_count, 0);
    }

    #[test]
    fn insert_then_lookup_round_trips_a_skipped_rewrite() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let original = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Skipped,
        };
        cache.insert(&key, &original);
        assert_eq!(cache.lookup(&key).expect("hit"), original);
    }

    #[test]
    fn insert_then_lookup_round_trips_the_entry() {
        let tmp = TempDir::new().expect("tempdir");
        let cache = cache_in(&tmp, 100);
        let key = CacheKey::compute(b"x = 1\n", CONFIG_A);
        let original = entry("y = 1\n");
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

//! User-level content-addressed cache for `prose check` and `prose format`.
//!
//! Keys are BLAKE3 digests over the source bytes, the active config's
//! canonical TOML, the resolved rule selection, the Prose version, the
//! private `CACHE_FORMAT_VERSION`, and the `Anchor` naming which
//! buffer the diagnostics resolve against, with a notebook's entry
//! also holding the code cells the run read. Entries live one file per
//! key under the platform's cache directory via `PROSE_CACHE_DIR` →
//! `dirs::cache_dir()`, inserts land through a temporary sibling and
//! `rename`, and LRU eviction by mtime sweeps once per run.

mod engine;
mod key;
mod records;

pub use engine::Cache;
pub use key::{Anchor, CacheKey, CacheKeyPrefix};
pub use records::{
    CacheEntry, CacheEntryRef, CacheInfo, CleanReport, NotebookCells, NotebookCellsRef,
    NotebookRewrite, Rewrite, RewriteKind,
};

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        time::{Duration, SystemTime},
    };

    use ruff_python_ast::PySourceType;
    use tempfile::TempDir;

    use super::key::CACHE_FORMAT_VERSION;
    use super::*;
    use crate::{
        diagnostics::Diagnostic,
        rule::RuleId,
        rules::{align_equals::AlignEquals, alphabetize_siblings::AlphabetizeSiblings},
        testing::{format_diagnostic, range},
        unstable::UnstableRewrite,
    };

    const CONFIG_A: &str = "code-line-length = 88\n";
    const CONFIG_B: &str = "code-line-length = 100\n";

    /// Backdates `dir` past the sweep's grace window.
    fn age(dir: &Path) {
        let stale = SystemTime::now() - Duration::from_hours(24);
        fs_err::File::open(dir)
            .expect("opens the directory")
            .set_modified(stale)
            .expect("backdates the directory");
    }
    #[test]
    fn key_for_separates_a_notebook_from_a_module_of_the_same_bytes() {
        let prefix = CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::AsWritten);
        assert_ne!(
            prefix.key_for(b"x = 1\n", PySourceType::Python),
            prefix.key_for(b"x = 1\n", PySourceType::Ipynb)
        );
    }

    fn cache_in(tmp: &TempDir, max_mib: u32) -> Cache {
        Cache {
            max_size_bytes: u64::from(max_mib) * 1024 * 1024,
            ..Cache::in_store(tmp.path().join("cache"))
        }
    }

    /// A fresh temp dir beside a 100 MiB cache rooted inside it.
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
    /// A generation directory beside the cache's own, holding one
    /// entry-shaped file.
    fn generation_beside(cache: &Cache, name: &str) -> std::path::PathBuf {
        let dir = cache.store.join(name);
        fs_err::create_dir_all(&dir).expect("creates");
        fs_err::write(dir.join("cafebabe"), b"an earlier build's entry").expect("writes");
        dir
    }

    /// Writes `entry` under `key` the way the run path does, borrowing
    /// the diagnostics and rewrite rather than handing over an owned
    /// record.
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

    /// The key one file draws under `config` and `selection`, the two
    /// steps the run path splits across a whole directory.
    fn key(
        source_bytes: &[u8],
        config_toml: &str,
        selection: impl IntoIterator<Item = RuleId>,
    ) -> CacheKey {
        CacheKeyPrefix::new(config_toml, selection, Anchor::AsWritten)
            .key_for(source_bytes, PySourceType::Python)
    }

    fn rules() -> [RuleId; 2] {
        [AlignEquals::SLUG, AlphabetizeSiblings::SLUG]
    }

    #[test]
    fn cache_key_differs_when_cache_format_version_changes() {
        let key_a = CacheKeyPrefix::with_versions(
            CONFIG_A,
            rules(),
            Anchor::AsWritten,
            env!("CARGO_PKG_VERSION"),
            "1",
        )
        .key_for(
            b"x = 1
",
            PySourceType::Python,
        );
        let key_b = CacheKeyPrefix::with_versions(
            CONFIG_A,
            rules(),
            Anchor::AsWritten,
            env!("CARGO_PKG_VERSION"),
            "2",
        )
        .key_for(
            b"x = 1
",
            PySourceType::Python,
        );
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_differs_when_config_changes() {
        let key_a = key(b"x = 1\n", CONFIG_A, rules());
        let key_b = key(b"x = 1\n", CONFIG_B, rules());
        assert_ne!(key_a, key_b);
        let key_c = key(b"x = 1\n", CONFIG_B, rules());
        assert_eq!(key_b, key_c);
    }

    #[test]
    fn cache_key_differs_when_prose_version_changes() {
        let key_a = CacheKeyPrefix::with_versions(
            CONFIG_A,
            rules(),
            Anchor::AsWritten,
            "0.2.3",
            CACHE_FORMAT_VERSION,
        )
        .key_for(
            b"x = 1
",
            PySourceType::Python,
        );
        let key_b = CacheKeyPrefix::with_versions(
            CONFIG_A,
            rules(),
            Anchor::AsWritten,
            "0.3.0",
            CACHE_FORMAT_VERSION,
        )
        .key_for(
            b"x = 1
",
            PySourceType::Python,
        );
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_differs_when_rule_selection_changes() {
        let key_a = key(b"x = 1\n", CONFIG_A, [AlignEquals::SLUG]);
        let key_b = key(b"x = 1\n", CONFIG_A, [AlphabetizeSiblings::SLUG]);
        assert_ne!(key_a, key_b);
        let key_c = key(b"x = 1\n", CONFIG_A, [AlignEquals::SLUG]);
        assert_eq!(key_a, key_c);
    }

    #[test]
    fn cache_key_differs_when_source_changes() {
        let key_a = key(b"x = 1\n", CONFIG_A, rules());
        let key_b = key(b"x = 2\n", CONFIG_A, rules());
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_differs_when_the_anchor_changes() {
        let as_written = CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::AsWritten);
        let rewritten = CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::Rewritten);
        assert_ne!(
            as_written.key_for(
                b"x = 1
",
                PySourceType::Python
            ),
            rewritten.key_for(
                b"x = 1
",
                PySourceType::Python
            )
        );
        assert_eq!(
            as_written.key_for(
                b"x = 1
",
                PySourceType::Python
            ),
            CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::AsWritten).key_for(
                b"x = 1
",
                PySourceType::Python
            ),
        );
    }

    #[test]
    fn cache_key_hex_renders_64_lowercase_chars() {
        let key = key(b"x = 1\n", CONFIG_A, rules());
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
            key(b"x = 1\n", CONFIG_A, rules()),
            key(b"x = 1\n", CONFIG_A, rules()),
        );
    }

    #[test]
    fn cache_key_separates_selections_that_concatenate_alike() {
        // Without the per-id delimiter both selections would feed the
        // hasher the same `abc` bytes and collide.
        let key_a = key(
            b"x = 1\n",
            CONFIG_A,
            [RuleId::from("ab"), RuleId::from("c")],
        );
        let key_b = key(
            b"x = 1\n",
            CONFIG_A,
            [RuleId::from("a"), RuleId::from("bc")],
        );
        assert_ne!(key_a, key_b);
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
        let report = cache.clean().expect("cleans");
        assert_eq!(report.entries, 0);
        assert_eq!(report.bytes, 0);
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
        let key_old = key(b"x = 1\n", CONFIG_A, rules());
        let key_new = key(b"y = 2\n", CONFIG_A, rules());
        insert(&cache, &key_old, &entry("a = 1\n"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        insert(&cache, &key_new, &entry("b = 2\n"));

        let tightened = Cache {
            max_size_bytes: 0,
            ..cache
        };
        let report = tightened.compact();

        assert!(report.entries >= 1);
        assert!(report.bytes > 0);
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
        insert(
            &cache,
            &key(b"x = 1\n", CONFIG_A, rules()),
            &entry("a = 1\n"),
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
        insert(
            &cache,
            &key(b"y = 2\n", CONFIG_A, rules()),
            &entry("b = 2\n"),
        );

        let capped = Cache {
            max_entries: 1,
            ..cache
        };
        let report = capped.compact();

        assert_eq!(report.entries, 1);
        assert_eq!(capped.info().entries, 1);
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
        let report = cache.compact();
        assert_eq!(report.entries, 0);
        assert_eq!(report.bytes, 0);
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
    fn info_and_compact_leave_the_own_output_ledger_alone() {
        let (_tmp, cache) = cached();
        cache.record_own_output(&key(b"x = 1\n", CONFIG_A, rules()));

        assert_eq!(cache.info().entries, 0);
        assert_eq!(cache.compact(), CleanReport::default());
        assert!(cache.owns_output(&key(b"x = 1\n", CONFIG_A, rules())));
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

    #[test]
    fn insert_then_lookup_round_trips_a_notebook_rewrite() {
        let (_tmp, cache) = cached();
        let key = key(b"nb", CONFIG_A, rules());
        let original = CacheEntry {
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
        };
        insert(&cache, &key, &original);
        assert_eq!(cache.lookup(&key).expect("hit"), original);
    }

    #[test]
    fn insert_then_lookup_round_trips_a_settle_report() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        let original = CacheEntry {
            unstable: Some(Box::new(UnstableRewrite {
                config_toml: CONFIG_A.to_owned(),
                first: "yy = 1\n".to_owned(),
                rules: vec![AlignEquals::SLUG],
                second: "yyy = 1\n".to_owned(),
            })),
            ..entry("yy = 1\n")
        };
        insert(&cache, &key, &original);

        assert_eq!(cache.lookup(&key).expect("hit"), original);
    }

    #[test]
    fn insert_then_lookup_round_trips_a_skipped_rewrite() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        let original = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::Skipped,
            unstable: None,
        };
        insert(&cache, &key, &original);
        assert_eq!(cache.lookup(&key).expect("hit"), original);
    }

    #[test]
    fn insert_then_lookup_round_trips_the_entry() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        let original = entry("y = 1\n");
        insert(&cache, &key, &original);
        let recovered = cache.lookup(&key).expect("hit");
        assert_eq!(recovered, original);
    }

    #[test]
    fn lookup_returns_none_for_corrupt_entry() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        fs_err::write(cache.path_for(&key), b"not postcard bytes").expect("writes");
        assert!(cache.lookup(&key).is_none());
    }

    #[test]
    fn lookup_returns_none_for_missing_entry() {
        let (_tmp, cache) = cached();
        let key = key(b"x = 1\n", CONFIG_A, rules());
        assert!(cache.lookup(&key).is_none());
    }
}

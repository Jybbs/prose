//! Tests of the result cache across runs and of the `cache` subcommand.

use std::path::Path;

use rstest::rstest;
use tempfile::tempdir;

use crate::{
    ALIGNS, CHECK_JSON, COLLAPSING_DICT, LINT_ONLY, TWO_CODE_CELLS, UNALIGNED, assert_stderr_has,
    assert_stdout_has, fixture, prose, prose_isolated, prose_with_cache, run_fixture,
    sibling_projects, stdout_utf8, summary_line, warmed_by, write_pyproject,
};

#[test]
fn cache_evicts_back_under_its_cap_once_a_run_ends() {
    let (dir, path) = fixture("a.py", UNALIGNED);
    write_pyproject(dir.path(), "[tool.prose.cache]\nmax-size-mib = 1\n");
    let (mut cmd, cache_dir) = prose_isolated();
    pad_with_entries(cache_dir.path());

    cmd.current_dir(dir.path())
        .arg("check")
        .arg(&path)
        .assert()
        .code(1);

    assert!(cache_bytes(cache_dir.path()) <= 1024 * 1024);
}

#[rstest]
#[case::a_module("ab.py", UNALIGNED)]
#[case::a_collapsing_literal("collapse.py", COLLAPSING_DICT)]
#[case::a_notebook("nb.ipynb", &TWO_CODE_CELLS)]
fn cache_hit_renders_like_a_cold_run(#[case] name: &str, #[case] source: &str) {
    let (_dir, path) = fixture(name, source);
    assert_warm_run_matches_cold(&[&path]);
}

#[test]
fn cache_hits_when_a_selection_is_repeated() {
    let (_dir, path) = fixture("repeat.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check", "--select", "align-equals"], 1);

    let assert = warm
        .args(["--verbose", "check", "--select", "align-equals"])
        .arg(&path)
        .assert();

    assert_stderr_has(&assert, "1 hits, 0 misses");
}

#[test]
fn cache_info_subcommand_prints_path_and_counts() {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", "info"]).assert().success();
    let out = stdout_utf8(&assert);
    assert!(out.contains("path:"), "stdout was {out:?}");
    assert!(out.contains("entries: 0"));
    assert!(out.contains("bytes: 0"));
}

#[test]
fn cache_invalidates_on_config_change() {
    let (project, py) = fixture("clean.py", "x = 1\n");
    let (mut warm, _cache) = warmed_by(&py, &["check"], 0);

    write_pyproject(project.path(), "[tool.prose]\ncode-line-length = 100\n");
    let assert = warm
        .args(["--verbose", "check"])
        .arg(&py)
        .assert()
        .success();
    assert_stderr_has(&assert, "0 hits, 1 misses");
}

#[test]
fn cache_keys_a_diff_apart_from_a_check() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, cache) = warmed_by(&path, &["check"], 1);
    warm.args(["format", "--diff"]).arg(&path).assert().code(1);

    let assert = prose_with_cache(cache.path())
        .args(["cache", "info"])
        .assert()
        .success();

    assert_stdout_has(&assert, "entries: 2");
}

#[test]
fn cache_keys_each_file_against_its_governing_config() {
    let parent = tempdir().expect("tempdir");
    let (suppressed, flagged) = sibling_projects(&parent, UNALIGNED);

    let out = assert_warm_run_matches_cold(&[&suppressed, &flagged]);

    let summary = summary_line(&out);
    assert_eq!(summary["files_visited"], 2);
    assert_eq!(summary["files_changed"], 1);
}

#[test]
fn cache_misses_a_diff_run_landing_on_a_check_entry() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    let assert = warm
        .args(["--verbose", "format", "--diff"])
        .arg(&path)
        .assert()
        .code(1);

    assert_stderr_has(&assert, "0 hits, 1 misses");
    assert_stdout_has(&assert, "@@");
}

#[rstest]
#[case::narrow_select_after_full_set(&[], &["--select", "alphabetize-siblings"])]
#[case::ignore_after_full_set(&[], &["--ignore", "align-equals"])]
#[case::full_set_after_narrow_select(&["--select", "alphabetize-siblings"], &[])]
fn cache_misses_when_selection_changes_between_runs(
    #[case] seed_filter: &[&str],
    #[case] query_filter: &[&str],
) {
    let (_dir, path) = fixture("reselect.py", UNALIGNED);
    assert_reselect_misses(seed_filter, query_filter, &path);
}

/// Seeds an isolated cache with one `format --diff` over a fresh fixture
/// holding `source`, then asserts the warm run reproduces the cold patch
/// byte for byte and reports the hit.
#[rstest]
#[case::a_module("misaligned.py", UNALIGNED)]
#[case::a_notebook("nb.ipynb", ALIGNS)]
fn cache_serves_a_warm_diff_from_its_own_entry(#[case] name: &str, #[case] source: &str) {
    let (_dir, path) = fixture(name, source);
    let (mut warm, _cache) = warmed_by(&path, &["format", "--diff"], 1);

    let hit = warm
        .args(["--verbose", "format", "--diff"])
        .arg(&path)
        .assert()
        .code(1);
    let cold = prose()
        .args(["format", "--diff", "--no-cache"])
        .arg(&path)
        .assert()
        .code(1);

    assert_eq!(
        hit.get_output().stdout,
        cold.get_output().stdout,
        "a warm diff must reproduce the cold patch byte for byte",
    );
    assert_stderr_has(&hit, "1 hits, 0 misses");
}

#[test]
fn cache_serves_a_warm_diff_of_surviving_lint() {
    let (_dir, path) = fixture("lint_only.py", LINT_ONLY);
    let (mut warm, _cache) = warmed_by(&path, &["format", "--diff"], 2);

    let hit = warm
        .args(["--verbose", "format", "--diff"])
        .arg(&path)
        .assert()
        .code(2);

    assert_stderr_has(&hit, "1 hits, 0 misses");
    assert_stderr_has(&hit, "lint diagnostic not shown");
}

#[test]
fn cache_stores_only_the_entry_a_write_back_leaves_reachable() {
    let (dir, _changed) = fixture("changed.py", UNALIGNED);
    fs_err::write(dir.path().join("settled.py"), "x = 1\n").expect("writes");
    let (mut cmd, cache_dir) = prose_isolated();
    cmd.arg("format").arg(dir.path()).assert().success();

    let assert = prose_with_cache(cache_dir.path())
        .args(["cache", "info"])
        .assert()
        .success();

    assert_stdout_has(&assert, "entries: 1");
}

#[rstest]
fn cache_subcommand_exits_zero_and_reports_count(#[values("clean", "compact")] subcommand: &str) {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", subcommand]).assert().success();
    let out = stdout_utf8(&assert);
    assert!(out.starts_with("removed "), "stdout was {out:?}");
    assert!(out.contains("entries"));
    assert!(out.contains("bytes"));
}

#[test]
fn cache_write_back_storing_nothing_leaves_an_over_cap_directory_alone() {
    let (dir, path) = fixture("a.py", UNALIGNED);
    write_pyproject(dir.path(), "[tool.prose.cache]\nmax-size-mib = 1\n");
    let (mut seed, cache_dir) = prose_isolated();
    seed.current_dir(dir.path())
        .arg("check")
        .arg(&path)
        .assert()
        .code(1);
    let generation = fs_err::read_dir(cache_dir.path())
        .expect("read_dir")
        .flatten()
        .find(|entry| entry.path().is_dir())
        .expect("a generation directory")
        .path();
    pad_with_entries(&generation);
    let padded = cache_bytes(&generation);
    fs_err::write(&path, UNALIGNED).expect("restores the unformatted bytes");

    prose_with_cache(cache_dir.path())
        .current_dir(dir.path())
        .arg("format")
        .arg(&path)
        .assert()
        .success();

    assert!(
        cache_bytes(&generation) >= padded,
        "a write-back run whose every rewrite went unstored must not sweep",
    );
    assert!(
        (0..4_u32).all(|slot| generation.join(format!("{slot:064x}")).exists()),
        "the padded entries survive, the run having stored nothing to evict for",
    );
}

#[test]
fn cache_writes_a_warm_rewrite_a_diff_run_recorded() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["format", "--diff"], 1);

    let assert = warm
        .args(["--verbose", "format"])
        .arg(&path)
        .assert()
        .success();

    assert_stderr_has(&assert, "1 hits, 0 misses");
    let (_cold_dir, cold) = fixture("misaligned.py", UNALIGNED);
    prose()
        .args(["format", "--no-cache"])
        .arg(&cold)
        .assert()
        .success();
    assert_eq!(
        fs_err::read_to_string(&path).expect("reads the warm rewrite"),
        fs_err::read_to_string(&cold).expect("reads the cold rewrite"),
    );
}

#[test]
fn cache_writes_back_a_settled_file_from_its_own_entry() {
    let (_dir, path) = fixture("settled.py", "x = 1\n");
    let (mut warm, _cache) = warmed_by(&path, &["format"], 0);

    let assert = warm
        .args(["--verbose", "format"])
        .arg(&path)
        .assert()
        .success();

    assert_stderr_has(&assert, "1 hits, 0 misses");
}

#[test]
fn verbose_flag_prints_cache_telemetry_to_stderr() {
    let assert = run_fixture("clean.py", "x = 1\n", &["--verbose", "check"]).success();

    assert_stderr_has(&assert, "cache:");
    assert_stderr_has(&assert, "files");
}

#[test]
fn verbose_flag_with_no_cache_reports_bypassed() {
    let assert =
        run_fixture("clean.py", "x = 1\n", &["--verbose", "check", "--no-cache"]).success();

    assert_stderr_has(&assert, "cache: bypassed");
}

/// Seeds an isolated cache by checking `path` under `seed_filter`, then
/// re-checks it under `query_filter` against the warm cache and asserts
/// the output matches a `--no-cache` run of the same query, so the
/// seed's selection never replays under the query's.
fn assert_reselect_misses(seed_filter: &[&str], query_filter: &[&str], path: &Path) {
    let (mut seed, cache_dir) = prose_isolated();
    let _ = seed.args(CHECK_JSON).args(seed_filter).arg(path).assert();

    let warm = prose_with_cache(cache_dir.path())
        .args(CHECK_JSON)
        .args(query_filter)
        .arg(path)
        .assert();
    let cold = prose()
        .args(CHECK_JSON)
        .arg("--no-cache")
        .args(query_filter)
        .arg(path)
        .assert();

    assert_eq!(
        warm.get_output().stdout,
        cold.get_output().stdout,
        "warm reselect must match a no-cache run",
    );
}

/// Runs `check` twice against one isolated cache, asserts the warm
/// run reproduces the cold stdout byte for byte, and returns it.
fn assert_warm_run_matches_cold(paths: &[&Path]) -> String {
    let (mut cold_cmd, cache_dir) = prose_isolated();
    let cold = cold_cmd.args(CHECK_JSON).args(paths).assert().code(1);
    let warm = prose_with_cache(cache_dir.path())
        .args(CHECK_JSON)
        .args(paths)
        .assert()
        .code(1);

    assert_eq!(cold.get_output().stdout, warm.get_output().stdout);
    stdout_utf8(&warm).to_owned()
}

/// Sums the bytes of every file in `dir`, the measure the cache's size
/// cap governs.
fn cache_bytes(dir: &Path) -> u64 {
    fs_err::read_dir(dir)
        .expect("read_dir")
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum()
}

/// Pads `dir` with four half-megabyte entries named the way a cache key
/// is.
fn pad_with_entries(dir: &Path) {
    let filler = vec![b'x'; 512 * 1024];
    for slot in 0..4_u32 {
        fs_err::write(dir.join(format!("{slot:064x}")), &filler).expect("writes");
    }
}

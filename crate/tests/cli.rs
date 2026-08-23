//! End-to-end tests against the `prose` binary, exercising
//! `cli::run` and the exit-code matrix.

use std::path::{Path, PathBuf};

use assert_cmd::{Command, assert::Assert};
use rstest::rstest;
use tempfile::{TempDir, tempdir};

/// Snapshot notebook fixtures the CLI tests reuse from the shared
/// tree, `ALIGNS` for the rewrite path, `EMPTY` for the no-op, and
/// `INTERLEAVED` for interspersed-Markdown cell numbering.
const ALIGNS: &str = include_str!("fixtures/notebook/code_cell_aligns/input.ipynb");
const EMPTY: &str = include_str!("fixtures/notebook/empty/input.ipynb");
const INTERLEAVED: &str = include_str!("fixtures/notebook/markdown_interleaved/input.ipynb");

/// A two-entry dict literal `reflow-collections` collapses, the shape
/// that net-shrinks the rewritten buffer.
const COLLAPSING_DICT: &str = "d = {\n    \"a\": 1,\n    \"b\": 2,\n}\n";

/// A Python notebook whose code cells each carry one assignment and a
/// trailing comment, so the rules covering them span a cell boundary.
const COMMENTED_CELLS: &str = r#"{
  "cells": [
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": ["x = 1  # a"]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": ["yy = 2  # bb"]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": ["zzz = 3  # ccc"]
    }
  ],
  "metadata": {
    "language_info": {"name": "python"}
  },
  "nbformat": 4,
  "nbformat_minor": 5
}"#;

/// A Python notebook whose sole code cell uses CRLF line endings. The
/// rewrite aligns the assignment while preserving each `\r\n`.
const CRLF_CELLS: &str = r#"{
  "cells": [
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": ["ab = 1\r\n", "x = 2\r\n"]
    }
  ],
  "metadata": {
    "language_info": {"name": "python"}
  },
  "nbformat": 4,
  "nbformat_minor": 5
}"#;

/// The aligned rewrite of `ESCAPE_IN_LITERAL`, its `U+001B` intact.
const ESCAPE_ALIGNED: &str = "AB = \"X\u{1b}Y\"\nc  = 2\n";

/// An unaligned assignment pair whose first value holds a literal `U+001B`.
const ESCAPE_IN_LITERAL: &str = "AB = \"X\u{1b}Y\"\nc = 2\n";

/// A two-cell notebook whose out-of-order imports sit in the first cell,
/// so the lone diagnostic falls in a non-last cell and the text emitter
/// renders it under that cell's header. The second cell reads both
/// modules bare, leaving the import lints quiet.
const FIRST_CELL_UNSORTED: &str = r#"{
  "cells": [
    {"cell_type": "code", "execution_count": null, "metadata": {}, "outputs": [], "source": ["import sys\n", "import os"]},
    {"cell_type": "code", "execution_count": null, "metadata": {}, "outputs": [], "source": ["value = 1\n", "print(os, sys)"]}
  ],
  "metadata": {
    "language_info": {"name": "python"}
  },
  "nbformat": 4,
  "nbformat_minor": 5
}"#;

/// A module whose bare `import os` draws a lint the formatter leaves
/// standing, so a `format` run discloses it beside the rewrite.
const LINT_ONLY: &str = "import os\nos.getcwd()\n";

/// An R-kernel notebook the formatter passes over, the way an excluded
/// path is skipped.
const NON_PYTHON: &str = r#"{
  "cells": [
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": ["x <- 1"]
    }
  ],
  "metadata": {
    "kernelspec": {"language": "R", "name": "ir"}
  },
  "nbformat": 4,
  "nbformat_minor": 5
}"#;

/// A `[tool.prose]` table disabling the rule the shared fixture
/// content fires, so a file governed by it checks clean.
const SUPPRESSING_PYPROJECT: &str = "[tool.prose.rules]\nalign-equals = false\n";

/// Two code cells, the second carrying a misaligned assignment pair so
/// its diagnostic ranges into a row the first cell pushes past in the
/// concatenated source, proving the report translates it cell-relative.
/// That cell also reads both modules bare, leaving the import lints
/// quiet.
const TWO_CODE_CELLS: &str = r#"{
  "cells": [
    {"cell_type": "code", "execution_count": null, "metadata": {}, "outputs": [], "source": ["import os\n", "import sys"]},
    {"cell_type": "code", "execution_count": null, "metadata": {}, "outputs": [], "source": ["x = 1\n", "yyy = 2\n", "print(os, sys)"]}
  ],
  "metadata": {
    "language_info": {"name": "python"}
  },
  "nbformat": 4,
  "nbformat_minor": 5
}"#;

/// The unaligned two-assignment source the reformat and config-resolution
/// tests reuse. `AB` is SCREAMING_CASE and `x` a single character, so the
/// lint rules pass it silently while `align-equals` still reshapes it.
const UNALIGNED: &str = "AB = 1\nx = 2\n";

/// A Python notebook whose sole code cell does not parse.
const UNPARSEABLE_CELL: &str = r#"{
  "cells": [
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": ["def f("]
    }
  ],
  "metadata": {
    "language_info": {"name": "python"}
  },
  "nbformat": 4,
  "nbformat_minor": 5
}"#;

fn assert_cache_hit_matches_miss(name: &str, source: &str) {
    let (_dir, path) = fixture(name, source);
    assert_warm_run_matches_cold(&[&path]);
}

fn assert_patch_keeps_escape(assert: &Assert) {
    assert_stdout_has(assert, "AB = \"X\u{1b}Y\"");
}

/// Seeds an isolated cache by checking `path` under `seed_filter`, then
/// re-checks it under `query_filter` against the warm cache and asserts
/// the output matches a `--no-cache` run of the same query, so the
/// seed's selection never replays under the query's.
fn assert_reselect_misses(seed_filter: &[&str], query_filter: &[&str], path: &Path) {
    let (mut seed, cache_dir) = prose_isolated();
    let _ = seed
        .args(["check", "--output-format", "json"])
        .args(seed_filter)
        .arg(path)
        .assert();

    let warm = prose()
        .args(["check", "--output-format", "json"])
        .args(query_filter)
        .arg(path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert();
    let cold = prose()
        .args(["check", "--no-cache", "--output-format", "json"])
        .args(query_filter)
        .arg(path)
        .assert();

    assert_eq!(
        warm.get_output().stdout,
        cold.get_output().stdout,
        "warm reselect must match a no-cache run",
    );
}

/// Asserts stderr carries `needle`, reporting the whole stream when it
/// does not.
fn assert_stderr_has(assert: &Assert, needle: &str) {
    let err = stderr_utf8(assert);
    assert!(err.contains(needle), "stderr was {err:?}");
}

/// Asserts stdout carries `needle`, reporting the whole stream when it
/// does not.
fn assert_stdout_has(assert: &Assert, needle: &str) {
    let out = stdout_utf8(assert);
    assert!(out.contains(needle), "stdout was {out:?}");
}

/// Seeds an isolated cache with one `format --diff` over a fresh fixture
/// holding `source`, then asserts the warm run reproduces the cold patch
/// byte for byte and reports the hit.
fn assert_warm_diff_matches_cold(name: &str, source: &str) {
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

/// Runs `check` twice against one isolated cache, asserts the warm
/// run reproduces the cold stdout byte for byte, and returns it.
fn assert_warm_run_matches_cold(paths: &[&Path]) -> String {
    let (mut cold_cmd, cache_dir) = prose_isolated();
    let cold = cold_cmd
        .args(["check", "--output-format", "json"])
        .args(paths)
        .assert()
        .code(1);
    let warm = prose()
        .args(["check", "--output-format", "json"])
        .args(paths)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .code(1);

    assert_eq!(cold.get_output().stdout, warm.get_output().stdout);
    stdout_utf8(&warm)
}

/// The byte total of every file in `dir`, the measure the cache's size
/// cap governs.
fn cache_bytes(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum()
}

/// Runs `check` with the JSON emitter over a fresh fixture holding
/// `source`, returning the summary line the run closed with.
fn check_json_summary(name: &str, source: &str, code: i32) -> serde_json::Value {
    let assert = run_fixture(name, source, &["check", "--output-format", "json"]).code(code);
    summary_line(&stdout_utf8(&assert))
}

fn fixture(name: &str, source: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    std::fs::write(&path, source).expect("writes");
    (dir, path)
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("valid JSON")
}

/// Frames each JSON-RPC body with its `Content-Length` header and joins
/// them into one stdio stream the language server can read end to end.
fn lsp_session(bodies: &[&str]) -> String {
    bodies
        .iter()
        .map(|body| format!("Content-Length: {}\r\n\r\n{body}", body.len()))
        .collect()
}

fn prose() -> Command {
    Command::cargo_bin("prose").expect("prose binary")
}

fn prose_isolated() -> (Command, TempDir) {
    let dir = tempdir().expect("tempdir");
    let mut cmd = prose();
    cmd.env("PROSE_CACHE_DIR", dir.path());
    (cmd, dir)
}

/// Runs `args` against a fresh fixture the way [`run_fixture`] does,
/// returning the assertion beside the file's contents after the run.
fn rewrite_fixture(name: &str, source: &str, args: &[&str]) -> (Assert, String) {
    let (_dir, path) = fixture(name, source);
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(args).arg(&path).assert();
    let after = std::fs::read_to_string(&path).expect("reads");
    (assert, after)
}

/// Runs `args` against a fresh fixture holding `source`, appending the
/// fixture's path last. The run reads a cache directory of its own, so
/// a case never inherits an entry another run left behind.
fn run_fixture(name: &str, source: &str, args: &[&str]) -> Assert {
    let (_dir, path) = fixture(name, source);
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(args).arg(&path).assert()
}

/// Runs `args` with `source` on stdin. The run reads a cache directory
/// of its own, so a case never reaches the user-level cache a developer
/// shares across projects.
fn run_stdin(source: &str, args: &[&str]) -> Assert {
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(args).write_stdin(source).assert()
}

/// Two sibling projects holding identical `source`: `suppressed/x.py`
/// under a config disabling `align-equals`, `flagged/y.py` under none.
fn sibling_projects(parent: &TempDir, source: &str) -> (PathBuf, PathBuf) {
    let suppressed = parent.path().join("suppressed");
    let flagged = parent.path().join("flagged");
    std::fs::create_dir_all(&suppressed).expect("dirs create");
    std::fs::create_dir_all(&flagged).expect("dirs create");
    write_pyproject(&suppressed, SUPPRESSING_PYPROJECT);
    let x = suppressed.join("x.py");
    let y = flagged.join("y.py");
    std::fs::write(&x, source).expect("writes");
    std::fs::write(&y, source).expect("writes");
    (x, y)
}

fn stderr_utf8(assert: &Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8")
}

fn stdout_utf8(assert: &Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8")
}

fn summary_line(out: &str) -> serde_json::Value {
    json(out.lines().last().expect("a summary line"))
}

/// A project directory whose config disables the rule the shared
/// fixture content fires.
fn suppressed_project() -> TempDir {
    let dir = tempdir().expect("tempdir");
    write_pyproject(dir.path(), SUPPRESSING_PYPROJECT);
    dir
}

/// Runs `seed` against `path` under a cache directory of its own,
/// asserting it closed at `code`, and returns a command bound to that
/// same warm cache for the follow-up run.
fn warmed_by(path: &Path, seed: &[&str], code: i32) -> (Command, TempDir) {
    let (mut cmd, cache_dir) = prose_isolated();
    cmd.args(seed).arg(path).assert().code(code);
    let mut warm = prose();
    warm.env("PROSE_CACHE_DIR", cache_dir.path());
    (warm, cache_dir)
}

fn write_pyproject(dir: &Path, contents: &str) {
    std::fs::write(dir.join("pyproject.toml"), contents).expect("writes pyproject");
}

#[test]
fn cache_clean_subcommand_exits_zero_and_reports_count() {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", "clean"]).assert().success();
    let out = stdout_utf8(&assert);
    assert!(out.starts_with("removed "), "stdout was {out:?}");
    assert!(out.contains("entries"));
    assert!(out.contains("bytes"));
}

#[test]
fn cache_compact_subcommand_exits_zero_and_reports_count() {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", "compact"]).assert().success();
    let out = stdout_utf8(&assert);
    assert!(out.starts_with("removed "), "stdout was {out:?}");
}

#[test]
fn cache_evicts_back_under_its_cap_once_a_run_ends() {
    let dir = tempdir().expect("tempdir");
    write_pyproject(dir.path(), "[tool.prose.cache]\nmax-size-mib = 1\n");
    let path = dir.path().join("a.py");
    std::fs::write(&path, UNALIGNED).expect("writes");
    let (mut cmd, cache_dir) = prose_isolated();
    let filler = vec![b'x'; 512 * 1024];
    for slot in 0..4_u32 {
        std::fs::write(cache_dir.path().join(format!("{slot:064x}")), &filler).expect("writes");
    }

    cmd.current_dir(dir.path())
        .arg("check")
        .arg(&path)
        .assert()
        .code(1);

    assert!(cache_bytes(cache_dir.path()) <= 1024 * 1024);
}

#[test]
fn cache_hit_produces_identical_diagnostics_to_miss() {
    assert_cache_hit_matches_miss("ab.py", UNALIGNED);
}

#[test]
fn cache_hit_renders_collapsing_literal_like_a_cold_run() {
    assert_cache_hit_matches_miss("collapse.py", COLLAPSING_DICT);
}

#[test]
fn cache_hits_when_a_selection_is_repeated() {
    let (_dir, path) = fixture("repeat.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check", "--select", "align-equals"], 1);

    let assert = warm
        .args(["--verbose", "check", "--select", "align-equals"])
        .arg(&path)
        .assert();

    assert!(
        stderr_utf8(&assert).contains("1 hits, 0 misses"),
        "the repeated selection must stay warm",
    );
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
    let project = tempdir().expect("project");
    let py = project.path().join("clean.py");
    std::fs::write(&py, "x = 1\n").expect("writes");
    let (mut warm_cmd, cache_dir) = prose_isolated();
    warm_cmd
        .args(["--verbose", "check"])
        .arg(&py)
        .current_dir(project.path())
        .assert()
        .success();

    write_pyproject(project.path(), "[tool.prose]\ncode-line-length = 100\n");
    let assert = prose()
        .args(["--verbose", "check"])
        .arg(&py)
        .current_dir(project.path())
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .success();
    assert_stderr_has(&assert, "0 hits, 1 misses");
}

#[test]
fn cache_keys_a_diff_apart_from_a_check() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, cache) = warmed_by(&path, &["check"], 1);
    warm.args(["format", "--diff"]).arg(&path).assert().code(1);

    let assert = prose()
        .args(["cache", "info"])
        .env("PROSE_CACHE_DIR", cache.path())
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

#[test]
fn cache_serves_a_warm_diff_from_its_own_entry() {
    assert_warm_diff_matches_cold("misaligned.py", UNALIGNED);
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
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("changed.py"), UNALIGNED).expect("writes");
    std::fs::write(dir.path().join("settled.py"), "x = 1\n").expect("writes");
    let (mut cmd, cache_dir) = prose_isolated();
    cmd.arg("format").arg(dir.path()).assert().success();

    let assert = prose()
        .args(["cache", "info"])
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .success();

    assert_stdout_has(&assert, "entries: 1");
}

#[test]
fn cache_write_back_storing_nothing_leaves_an_over_cap_directory_alone() {
    let dir = tempdir().expect("tempdir");
    write_pyproject(dir.path(), "[tool.prose.cache]\nmax-size-mib = 1\n");
    let path = dir.path().join("a.py");
    std::fs::write(&path, UNALIGNED).expect("writes");
    let (mut seed, cache_dir) = prose_isolated();
    seed.current_dir(dir.path())
        .arg("check")
        .arg(&path)
        .assert()
        .code(1);
    let generation = std::fs::read_dir(cache_dir.path())
        .expect("read_dir")
        .flatten()
        .find(|entry| entry.path().is_dir())
        .expect("a generation directory")
        .path();
    let filler = vec![b'x'; 512 * 1024];
    for slot in 0..4_u32 {
        std::fs::write(generation.join(format!("{slot:064x}")), &filler).expect("writes");
    }
    let padded = cache_bytes(&generation);
    std::fs::write(&path, UNALIGNED).expect("restores the unformatted bytes");

    prose()
        .current_dir(dir.path())
        .arg("format")
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
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
        std::fs::read_to_string(&path).expect("reads the warm rewrite"),
        std::fs::read_to_string(&cold).expect("reads the cold rewrite"),
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
fn check_clean_fixture_exits_zero() {
    run_fixture("clean.py", "x = 1\n", &["check"]).success();
}

#[test]
fn check_clean_summary_anchors_with_hyacinth() {
    let assert = run_fixture("clean.py", "x = 1\n", &["check"]).success();

    assert_eq!(stderr_utf8(&assert).trim(), "🪻 All clean.");
}

#[test]
fn check_dash_clean_exits_zero() {
    run_stdin("x = 1\n", &["check", "-"]).success();
}

#[test]
fn check_dash_unaligned_exits_format_change() {
    run_stdin(UNALIGNED, &["check", "-"]).code(1);
}

#[test]
fn check_file_in_another_project_draws_its_own_config() {
    let cwd_project = suppressed_project();
    let (_dir, path) = fixture("unaligned.py", UNALIGNED);

    prose()
        .args(["check", "--no-cache"])
        .arg(&path)
        .current_dir(cwd_project.path())
        .assert()
        .code(1);
}

#[test]
fn check_json_closes_clean_run_with_summary_envelope() {
    let summary = check_json_summary("clean.py", "x = 1\n", 0);

    assert_eq!(summary["kind"], "summary");
    assert_eq!(summary["diagnostics_total"], 0);
    assert_eq!(summary["files_visited"], 1);
    assert_eq!(summary["files_changed"], 0);
}

#[test]
fn check_json_counts_a_collapsing_literal_as_changed() {
    let summary = check_json_summary("collapse.py", COLLAPSING_DICT, 1);

    assert_eq!(summary["files_changed"], 1);
}

#[test]
fn check_json_summary_counts_a_changed_file() {
    let summary = check_json_summary("misaligned.py", UNALIGNED, 1);

    assert_eq!(summary["kind"], "summary");
    assert_eq!(summary["files_visited"], 1);
    assert_eq!(summary["files_changed"], 1);
    assert!(
        summary["diagnostics_total"].as_u64().expect("integer") >= 1,
        "diagnostics_total was {:?}",
        summary["diagnostics_total"],
    );
    assert!(
        !summary["rules_fired"]
            .as_object()
            .expect("object")
            .is_empty(),
        "rules_fired was {:?}",
        summary["rules_fired"],
    );
}

#[test]
fn check_no_cache_flag_runs_clean() {
    run_fixture("clean.py", "x = 1\n", &["check", "--no-cache"]).success();
}

#[test]
fn check_ordinary_module_prunes_its_unread_import() {
    run_fixture("mod.py", "import numpy as np\n", &["check", "--no-cache"]).code(1);
}

#[test]
fn check_package_init_reports_its_unread_import() {
    run_fixture(
        "__init__.py",
        "import numpy as np\n",
        &["check", "--no-cache"],
    )
    .code(2);
}

#[test]
fn check_relative_path_resolves_its_ancestor_config() {
    let project = suppressed_project();
    std::fs::write(project.path().join("unaligned.py"), UNALIGNED).expect("writes");

    prose()
        .args(["check", "--no-cache", "unaligned.py"])
        .current_dir(project.path())
        .assert()
        .success();
}

#[test]
fn check_resolves_each_files_config_from_its_own_project() {
    let parent = tempdir().expect("tempdir");
    let (suppressed, flagged) = sibling_projects(&parent, UNALIGNED);

    let assert = prose()
        .args(["check", "--no-cache", "--output-format", "json"])
        .args([&suppressed, &flagged])
        .assert()
        .code(1);

    let out = stdout_utf8(&assert);
    let diagnostics: Vec<serde_json::Value> = out
        .lines()
        .map(json)
        .filter(|record: &serde_json::Value| record["kind"] != "summary")
        .collect();
    assert!(!diagnostics.is_empty(), "stdout was {out:?}");
    for diagnostic in &diagnostics {
        let filename = diagnostic["filename"].as_str().expect("a filename");
        assert!(filename.ends_with("y.py"), "flagged {filename:?}");
    }
}

#[test]
fn check_respects_cache_disabled_in_pyproject() {
    let project = tempdir().expect("tempdir");
    write_pyproject(project.path(), "[tool.prose.cache]\nenabled = false\n");
    let py = project.path().join("clean.py");
    std::fs::write(&py, "x = 1\n").expect("writes");
    let (mut cmd, _cache_dir) = prose_isolated();

    let assert = cmd
        .args(["--verbose", "check"])
        .arg(&py)
        .current_dir(project.path())
        .assert()
        .success();

    assert_stderr_has(&assert, "cache: bypassed");
}

#[test]
fn check_stdin_clean_exits_zero() {
    run_stdin("x = 1\n", &["check", "--stdin"]).success();
}

#[test]
fn check_stdin_resolves_config_from_the_cwd() {
    let project = suppressed_project();
    let (mut cmd, _cache_dir) = prose_isolated();

    cmd.args(["check", "--stdin"])
        .write_stdin(UNALIGNED)
        .current_dir(project.path())
        .assert()
        .success();
}

#[test]
fn check_stdin_unaligned_exits_format_change() {
    run_stdin(UNALIGNED, &["check", "--stdin"]).code(1);
}

#[test]
fn check_unaligned_fixture_exits_format_change() {
    run_fixture("unaligned.py", UNALIGNED, &["check"]).code(1);
}

#[test]
fn check_unparseable_fixture_exits_parse_error() {
    run_fixture("broken.py", "def x(:", &["check"]).code(3);
}

#[test]
fn check_validate_bypasses_a_check_populated_cache_entry() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    let assert = warm
        .args(["check", "--validate", "--verbose"])
        .arg(&path)
        .assert()
        .code(1);

    assert_stderr_has(&assert, "0 hits, 1 misses");
}

#[test]
fn check_validate_flag_accepts_a_valid_rewrite() {
    run_fixture("unaligned.py", UNALIGNED, &["check", "--validate"]).code(1);
}

#[test]
fn check_violation_summary_anchors_with_bookmark() {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["check"]).code(1);

    assert_stderr_has(&assert, "🔖 1 diagnostic in 1 file.");
}

#[test]
fn check_warns_a_precedence_note_once_across_both_config_loads() {
    let project = tempdir().expect("tempdir");
    std::fs::write(project.path().join("prose.toml"), "code-line-length = 90\n")
        .expect("writes prose.toml");
    write_pyproject(project.path(), "[tool.prose]\ncode-line-length = 100\n");
    std::fs::write(project.path().join("a.py"), "x = 1\n").expect("writes");

    let assert = prose()
        .args(["check", "--no-cache", "a.py"])
        .current_dir(project.path())
        .assert()
        .success();

    let err = stderr_utf8(&assert);
    assert_eq!(
        err.matches("takes precedence over").count(),
        1,
        "the precedence note must appear once, stderr was {err:?}",
    );
}

#[test]
fn color_always_summary_emits_truecolor_when_colorterm_set() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();

    let assert = cmd
        .env("COLORTERM", "truecolor")
        .args(["--color", "always", "check"])
        .arg(&path)
        .assert()
        .success();

    assert_stderr_has(&assert, "\u{1b}[38;2;138;128;203m");
}

#[test]
fn color_always_summary_falls_back_to_ansi_without_colorterm() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();

    let assert = cmd
        .env_remove("COLORTERM")
        .args(["--color", "always", "check"])
        .arg(&path)
        .assert()
        .success();

    let err = stderr_utf8(&assert);
    assert!(err.contains("\u{1b}[35m"), "stderr was {err:?}");
    assert!(!err.contains("38;2;"), "stderr was {err:?}");
}

#[rstest]
#[case::always("always", true)]
#[case::never("never", false)]
fn color_arm_paints_the_diagnostic_body(#[case] arm: &str, #[case] painted: bool) {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["--color", arm, "check"]).code(1);

    let out = stdout_utf8(&assert);
    assert_eq!(out.contains('\u{1b}'), painted, "stdout was {out:?}");
}

#[rstest]
fn color_arms_exit_zero(#[values("always", "never")] arm: &str) {
    run_fixture("clean.py", "x = 1\n", &["--color", arm, "check"]).success();
}

#[test]
fn color_never_summary_stays_plain() {
    let assert = run_fixture("clean.py", "x = 1\n", &["--color", "never", "check"]).success();

    let err = stderr_utf8(&assert);
    assert!(!err.contains('\u{1b}'), "stderr was {err:?}");
}

#[test]
fn completions_bash_exits_zero() {
    prose().args(["completions", "bash"]).assert().success();
}

#[rstest]
#[case(&["check", "--stdin", "."])]
#[case(&["check", "-", "--stdin"])]
#[case(&["check", "-", "a.py"])]
#[case(&["--not-a-flag"])]
#[case(&["check", "--select", "not-a-rule", "."])]
#[case(&["format", "--diff", "--output-format", "json", "."])]
fn config_errors_exit_four(#[case] args: &[&str]) {
    prose().args(args).assert().code(4);
}

#[rstest]
fn cwd_config_error_exits_four(#[values("check", "format")] subcommand: &str) {
    let project = tempdir().expect("tempdir");
    write_pyproject(project.path(), "[this is not valid TOML\n");
    std::fs::write(project.path().join("a.py"), "x = 1\n").expect("writes");

    prose()
        .args([subcommand, "--no-cache", "a.py"])
        .current_dir(project.path())
        .assert()
        .code(4);
}

/// Each input drives a rule that net-shrinks the buffer (`reflow-collections`
/// collapsing or re-laying-out a literal), the shape that overran the
/// rewritten buffer before reporting anchored to the source as written. A
/// panic in the binary would surface as exit code 101, not the format-change 1.
#[rstest]
#[case::two_entry_dict(COLLAPSING_DICT)]
#[case::three_entry_list("XS = [\n    1,\n    2,\n    3,\n]\n")]
#[case::noncollapsible_call_dict(
    "config = {\n        \"alpha\": build_widget(first_argument, second_argument, third_argument),\n        \"beta\": build_gadget(fourth_argument, fifth_argument, sixth_argument),\n}\n"
)]
fn emitters_render_shrinking_literals_without_aborting(
    #[case] source: &str,
    #[values("text", "json")] format: &str,
) {
    run_fixture("literal.py", source, &["check", "--output-format", format]).code(1);
}

#[test]
fn format_dash_keeps_escape_bytes_in_piped_stdout() {
    run_stdin(ESCAPE_IN_LITERAL, &["format", "-"])
        .success()
        .stdout(ESCAPE_ALIGNED);
}

#[test]
fn format_dash_prints_canonical_source_verbatim() {
    run_stdin("x = 1\n", &["format", "-"])
        .success()
        .stdout("x = 1\n");
}

#[test]
fn format_dash_rewrites_unaligned_stdin_to_stdout() {
    run_stdin(UNALIGNED, &["format", "-"])
        .success()
        .stdout("AB = 1\nx  = 2\n");
}

#[test]
fn format_diff_keeps_escape_bytes_in_a_plain_patch() {
    let assert = run_fixture("escape.py", ESCAPE_IN_LITERAL, &["format", "--diff"]).code(1);

    assert_patch_keeps_escape(&assert);
}

#[test]
fn format_diff_off_tty_leaves_a_plain_patch() {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["format", "--diff"]).code(1);

    let stdout = stdout_utf8(&assert);
    assert!(stdout.contains("--- "), "patch header missing: {stdout:?}");
    assert!(
        !stdout.contains('🧵'),
        "decoration leaked off a TTY: {stdout:?}"
    );
}

#[test]
fn format_diff_renders_diff_and_leaves_file_unchanged() {
    let (assert, after) = rewrite_fixture("unaligned.py", UNALIGNED, &["format", "--diff"]);
    let assert = assert.code(1);

    let stdout = stdout_utf8(&assert);
    assert!(stdout.contains("@@"), "diff missing hunks: {stdout:?}");
    assert!(
        stdout.contains("-x = 2"),
        "diff missing before line: {stdout:?}"
    );
    assert!(
        stdout.contains("+x  = 2"),
        "diff missing after line: {stdout:?}"
    );
    assert_eq!(after, UNALIGNED);
}

#[test]
fn format_diff_summary_reports_would_reformat() {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["format", "--diff"]).code(1);

    assert_stderr_has(&assert, "🗞️ 1 file would be reformatted.");
}

#[test]
fn format_discloses_surviving_lint_beside_a_rewrite() {
    let assert = run_fixture(
        "both.py",
        &format!("import os\n\n{UNALIGNED}\nos.getcwd()\n"),
        &["format", "--no-cache"],
    )
    .code(2);

    assert_stderr_has(&assert, "🗞️ Reformatted 1 file.");
    assert_stderr_has(
        &assert,
        "🔖 1 lint diagnostic not shown. Run `prose check` to see it in full.",
    );
}

#[test]
fn format_discloses_surviving_lint_when_nothing_reformats() {
    let assert = run_fixture("lint_only.py", LINT_ONLY, &["format", "--no-cache"]).code(2);

    assert_stderr_has(
        &assert,
        "🔖 1 lint diagnostic not shown. Run `prose check` to see it in full.",
    );
    let err = stderr_utf8(&assert);
    assert!(!err.contains("All clean"), "the clean line leaked: {err:?}");
}

#[test]
fn format_json_renders_collapsing_literal_without_aborting() {
    let assert = run_fixture(
        "collapse.py",
        COLLAPSING_DICT,
        &["format", "--output-format", "json"],
    )
    .success();

    assert_stdout_has(&assert, "reflow-collections");
}

#[test]
fn format_json_rewrites_over_a_check_cache_entry() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    let assert = warm
        .args(["format", "--output-format", "json"])
        .arg(&path)
        .assert()
        .success();

    let after = std::fs::read_to_string(&path).expect("reads");
    assert_ne!(after, UNALIGNED);
    assert_stdout_has(&assert, "align-equals");
}

#[test]
fn format_json_with_surviving_lint_exits_two_without_a_stderr_disclosure() {
    let assert = run_fixture(
        "lint_only.py",
        LINT_ONLY,
        &["format", "--no-cache", "--output-format", "json"],
    )
    .code(2);

    assert_stdout_has(&assert, "\"code\"");
    let err = stderr_utf8(&assert);
    assert!(
        !err.contains("lint diagnostic not shown"),
        "a structured run must not repeat the lint on stderr: {err:?}",
    );
}

#[test]
fn format_keeps_escape_bytes_in_the_rewritten_file() {
    let (assert, after) = rewrite_fixture("escape.py", ESCAPE_IN_LITERAL, &["format"]);
    assert.success();

    assert_eq!(after, ESCAPE_ALIGNED);
}

#[test]
fn format_leaves_no_bug_notice_on_a_settling_rewrite() {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["format"]).success();

    let err = stderr_utf8(&assert);
    assert!(!err.contains('🐞'), "stderr was {err:?}");
    assert!(err.contains("🗞️ Reformatted 1 file."), "stderr was {err:?}");
}

#[test]
fn format_no_cache_flag_rewrites_when_needed() {
    let (assert, after) = rewrite_fixture("unaligned.py", UNALIGNED, &["format", "--no-cache"]);
    assert.success();

    assert_ne!(after, UNALIGNED);
}

#[test]
fn format_rewrite_summary_reports_reformatted() {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["format"]).success();

    assert_stderr_has(&assert, "🗞️ Reformatted 1 file.");
}

#[test]
fn format_rewrites_after_check_populated_the_cache() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    warm.arg("format").arg(&path).assert().success();

    let after = std::fs::read_to_string(&path).expect("reads");
    assert_ne!(after, UNALIGNED);
}

#[test]
fn format_stdin_diff_keeps_escape_bytes_in_a_plain_patch() {
    let assert = run_stdin(ESCAPE_IN_LITERAL, &["format", "--stdin", "--diff"]).code(1);

    assert_patch_keeps_escape(&assert);
}

#[test]
fn format_stdin_resolves_config_from_the_cwd() {
    let project = suppressed_project();
    let (mut cmd, _cache_dir) = prose_isolated();

    cmd.args(["format", "--stdin"])
        .write_stdin(UNALIGNED)
        .current_dir(project.path())
        .assert()
        .success()
        .stdout(UNALIGNED);
}

#[test]
fn format_unaligned_rewrites_and_re_check_is_clean() {
    let (_dir, path) = fixture("unaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["format"], 0);

    warm.arg("check").arg(&path).assert().success();
}

#[test]
fn format_warns_an_unknown_key_once_across_both_config_loads() {
    let project = tempdir().expect("tempdir");
    write_pyproject(project.path(), "[tool.prose]\nmax-shft = 4\n");
    let py = project.path().join("a.py");
    std::fs::write(&py, UNALIGNED).expect("writes");

    let assert = prose()
        .args(["format", "--no-cache"])
        .arg(&py)
        .current_dir(project.path())
        .assert()
        .success();

    let err = stderr_utf8(&assert);
    assert_eq!(
        err.matches("unknown key `max-shft`").count(),
        1,
        "the unknown key must warn once, stderr was {err:?}",
    );
}

#[test]
fn help_exits_clean() {
    prose().arg("--help").assert().success();
}

#[test]
fn no_args_prints_help_and_exits_clean() {
    prose().assert().success();
}

#[test]
fn notebook_check_hit_renders_like_a_cold_run() {
    assert_cache_hit_matches_miss("nb.ipynb", TWO_CODE_CELLS);
}

#[test]
fn notebook_check_json_renders_cell_relative_with_cell_number() {
    let assert = run_fixture(
        "nb.ipynb",
        TWO_CODE_CELLS,
        &["check", "--no-cache", "--output-format", "json"],
    )
    .code(1);

    let out = stdout_utf8(&assert);
    let record = json(out.lines().next().expect("a diagnostic line"));
    assert_eq!(
        record["cell"], 2,
        "diagnostic carries its absolute cell number"
    );
    assert_eq!(
        record["location"]["row"], 1,
        "the second cell's diagnostic translates to a cell-relative row",
    );
}

#[test]
fn notebook_check_reports_the_align_diagnostic() {
    let assert = run_fixture("nb.ipynb", ALIGNS, &["check", "--no-cache"]).code(1);

    assert_stdout_has(&assert, "align-equals");
}

#[test]
fn notebook_check_survives_a_cache_round_trip() {
    let (_dir, path) = fixture("nb.ipynb", ALIGNS);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    let assert = warm
        .args(["--verbose", "check"])
        .arg(&path)
        .assert()
        .code(1);

    assert!(
        stderr_utf8(&assert).contains("1 hits"),
        "the second run should rehydrate from the cache",
    );
}

#[test]
fn notebook_check_text_renders_a_cell_header_off_a_hit() {
    let (_dir, path) = fixture("nb.ipynb", TWO_CODE_CELLS);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    let assert = warm.arg("check").arg(&path).assert().code(1);

    assert_stdout_has(&assert, "cell 2");
}

#[test]
fn notebook_check_text_renders_a_diagnostic_spanning_two_cells() {
    let assert = run_fixture("nb.ipynb", COMMENTED_CELLS, &["check", "--no-cache"]).code(1);

    assert_stdout_has(&assert, "cells 1 to 2");
}

#[test]
fn notebook_check_text_renders_a_non_last_cell_under_its_header() {
    let assert = run_fixture("nb.ipynb", FIRST_CELL_UNSORTED, &["check", "--no-cache"]).code(1);

    let out = stdout_utf8(&assert);
    assert!(
        out.contains("cell 1"),
        "non-last cell header missing: {out:?}"
    );
}

#[test]
fn notebook_check_text_renders_under_a_cell_header() {
    let assert = run_fixture("nb.ipynb", TWO_CODE_CELLS, &["check", "--no-cache"]).code(1);

    assert_stdout_has(&assert, "cell 2");
}

#[test]
fn notebook_check_validate_reports_the_pending_change() {
    run_fixture("nb.ipynb", ALIGNS, &["check", "--validate", "--no-cache"]).code(1);
}

#[test]
fn notebook_diff_numbers_interleaved_cells_by_absolute_position() {
    let assert = run_fixture("nb.ipynb", INTERLEAVED, &["format", "--diff", "--no-cache"]).code(1);

    let stdout = stdout_utf8(&assert);
    assert!(
        stdout.contains("cell 2"),
        "first code cell missing: {stdout:?}"
    );
    assert!(
        stdout.contains("cell 5"),
        "interspersed code cell mis-numbered: {stdout:?}"
    );
    assert!(
        !stdout.contains("cell 4"),
        "unchanged cell rendered: {stdout:?}"
    );
    assert!(
        !stdout.contains("cell 1") && !stdout.contains("cell 3"),
        "code-cell ordinal leaked: {stdout:?}"
    );
}

#[test]
fn notebook_diff_renders_per_cell_hunks() {
    let assert = run_fixture("nb.ipynb", ALIGNS, &["format", "--diff", "--no-cache"]).code(1);

    let stdout = stdout_utf8(&assert);
    assert!(
        stdout.contains("cell 2"),
        "absolute cell header missing: {stdout:?}"
    );
    assert!(
        !stdout.contains("cell 1"),
        "code-cell ordinal leaked: {stdout:?}"
    );
    assert!(stdout.contains("-x = 1"), "before line missing: {stdout:?}");
    assert!(stdout.contains("+x  = 1"), "after line missing: {stdout:?}");
}

#[test]
fn notebook_diff_renders_per_cell_hunks_from_a_warm_entry() {
    assert_warm_diff_matches_cold("nb.ipynb", ALIGNS);
}

#[test]
fn notebook_discovered_in_a_directory_walk() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("nb.ipynb"), ALIGNS).expect("writes");

    prose()
        .args(["format", "--no-cache"])
        .arg(dir.path())
        .assert()
        .success();

    let after = std::fs::read_to_string(dir.path().join("nb.ipynb")).expect("reads");
    assert_eq!(json(&after)["cells"][1]["source"][0], "x  = 1\n");
}

#[test]
fn notebook_empty_is_a_clean_no_op() {
    run_fixture("nb.ipynb", EMPTY, &["format", "--no-cache"]).success();
}

#[test]
fn notebook_format_is_idempotent() {
    let (_dir, path) = fixture("nb.ipynb", ALIGNS);
    let format = || prose().args(["format", "--no-cache"]).arg(&path).assert();
    format().success();
    let once = std::fs::read_to_string(&path).expect("reads");
    format().success();
    let twice = std::fs::read_to_string(&path).expect("reads");
    assert_eq!(once, twice);
}

#[test]
fn notebook_format_preserves_crlf_line_endings() {
    let (assert, after) = rewrite_fixture("nb.ipynb", CRLF_CELLS, &["format", "--no-cache"]);
    assert.success();

    assert_eq!(json(&after)["cells"][0]["source"][1], "x  = 2\r\n");
}

#[test]
fn notebook_format_preserves_outputs_and_rewrites_code() {
    let (assert, after) = rewrite_fixture("nb.ipynb", ALIGNS, &["format", "--no-cache"]);
    assert.success();

    let parsed = json(&after);
    assert_eq!(parsed["cells"][1]["source"][0], "x  = 1\n");
    assert_eq!(parsed["cells"][1]["execution_count"], 3);
    assert_eq!(parsed["cells"][1]["outputs"][0]["text"][0], "2\n");
}

#[test]
fn notebook_malformed_json_exits_parse_error() {
    run_fixture("bad.ipynb", "{not valid json", &["check", "--no-cache"]).code(3);
}

#[test]
fn notebook_non_python_is_passed_over() {
    run_fixture("nb.ipynb", NON_PYTHON, &["check", "--no-cache"]).success();
}

#[test]
fn notebook_non_python_through_stdin_is_echoed_verbatim() {
    let assert = run_stdin(
        NON_PYTHON,
        &["format", "--stdin", "--stdin-filename", "x.ipynb"],
    )
    .success();

    assert_eq!(stdout_utf8(&assert), NON_PYTHON);
}

#[test]
fn notebook_stdin_diff_numbers_by_absolute_cell() {
    let assert = run_stdin(
        ALIGNS,
        &[
            "format",
            "--diff",
            "--stdin",
            "--stdin-filename",
            "nb.ipynb",
        ],
    )
    .code(1);

    assert_stdout_has(&assert, "cell 2");
}

#[test]
fn notebook_stdin_filename_selects_the_notebook_type() {
    let assert = run_stdin(
        ALIGNS,
        &["format", "--stdin", "--stdin-filename", "x.ipynb"],
    )
    .success();

    assert_eq!(
        json(&stdout_utf8(&assert))["cells"][1]["source"][0],
        "x  = 1\n"
    );
}

#[test]
fn notebook_unparseable_cell_exits_parse_error() {
    run_fixture("nb.ipynb", UNPARSEABLE_CELL, &["check", "--no-cache"]).code(3);
}

#[test]
fn quiet_check_reduces_summary_to_a_bare_count() {
    let (_dir, path) = fixture("unaligned.py", UNALIGNED);
    let (mut cmd, _cache_dir) = prose_isolated();

    let assert = cmd
        .env("COLORTERM", "truecolor")
        .args(["--color", "always", "check", "--quiet"])
        .arg(&path)
        .assert()
        .code(1);

    let err = stderr_utf8(&assert);
    assert_eq!(err.trim(), "1 diagnostic in 1 file.");
    assert!(!err.contains('🔖'), "quiet kept the anchor: {err:?}");
    assert!(!err.contains('\u{1b}'), "quiet kept color: {err:?}");
}

#[test]
fn rules_json_lists_every_registered_rule_in_pipeline_order() {
    let assert = prose()
        .args(["rules", "--output-format", "json"])
        .assert()
        .success();

    let rules = json(&stdout_utf8(&assert));

    insta::assert_snapshot!(serde_json::to_string_pretty(&rules).expect("renders"));
}

#[test]
fn schema_subcommand_exits_zero_and_prints_the_schema() {
    let assert = prose().arg("schema").assert().success();

    assert!(json(&stdout_utf8(&assert))["properties"]["rules"].is_object());
}

#[test]
fn server_completes_a_stdio_session_over_the_real_binary() {
    let session = lsp_session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"untitled:m.py","languageId":"python","version":1,"text":"import os\nos.getcwd()\n"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":"untitled:m.py"},"options":{"tabSize":4,"insertSpaces":true}}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#,
        r#"{"jsonrpc":"2.0","method":"exit","params":null}"#,
    ]);

    let assert = prose()
        .arg("server")
        .write_stdin(session)
        .assert()
        .success();

    let out = stdout_utf8(&assert);
    assert!(
        out.contains("documentFormattingProvider"),
        "initialize result missing capabilities: {out:?}",
    );
    assert!(
        out.contains("publishDiagnostics") && out.contains("bare-imports"),
        "diagnostics not published: {out:?}",
    );
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

#[test]
fn version_exits_clean() {
    prose().arg("--version").assert().success();
}

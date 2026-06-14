//! End-to-end tests against the `prose` binary, exercising
//! `cli::run` and the exit-code matrix.

use std::{
    fs::write,
    path::{Path, PathBuf},
};

use assert_cmd::{Command, assert::Assert};
use rstest::rstest;
use tempfile::{TempDir, tempdir};

/// A `[tool.prose]` table disabling the rule the shared fixture
/// content fires, so a file governed by it checks clean.
const SUPPRESSING_PYPROJECT: &str = "[tool.prose.rules]\nalign-equals = false\n";

fn assert_cache_hit_matches_miss(name: &str, source: &str) {
    let (_dir, path) = fixture(name, source);
    assert_warm_run_matches_cold(&[&path]);
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

fn fixture(name: &str, source: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    write(&path, source).expect("writes");
    (dir, path)
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

/// Two sibling projects holding identical `source`: `suppressed/x.py`
/// under a config disabling `align-equals`, `flagged/y.py` under none.
fn sibling_projects(parent: &TempDir, source: &str) -> (PathBuf, PathBuf) {
    let suppressed = parent.path().join("suppressed");
    let flagged = parent.path().join("flagged");
    std::fs::create_dir_all(&suppressed).expect("dirs create");
    std::fs::create_dir_all(&flagged).expect("dirs create");
    write(suppressed.join("pyproject.toml"), SUPPRESSING_PYPROJECT).expect("writes pyproject");
    let x = suppressed.join("x.py");
    let y = flagged.join("y.py");
    write(&x, source).expect("writes");
    write(&y, source).expect("writes");
    (x, y)
}

fn stderr_utf8(assert: &Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8")
}

fn stdout_utf8(assert: &Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8")
}

fn summary_line(out: &str) -> serde_json::Value {
    serde_json::from_str(out.lines().last().expect("a summary line")).expect("parses")
}

/// A project directory whose config disables the rule the shared
/// fixture content fires.
fn suppressed_project() -> TempDir {
    let dir = tempdir().expect("tempdir");
    write(dir.path().join("pyproject.toml"), SUPPRESSING_PYPROJECT).expect("writes pyproject");
    dir
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
fn cache_hit_produces_identical_diagnostics_to_miss() {
    assert_cache_hit_matches_miss("ab.py", "ab = 1\nx = 2\n");
}

#[test]
fn cache_hit_renders_collapsing_literal_like_a_cold_run() {
    assert_cache_hit_matches_miss("collapse.py", "d = {\n    \"a\": 1,\n    \"b\": 2,\n}\n");
}

#[test]
fn cache_hits_when_a_selection_is_repeated() {
    let (_dir, path) = fixture("repeat.py", "ab = 1\nx = 2\n");
    let (mut cold, cache_dir) = prose_isolated();
    let _ = cold
        .args(["check", "--select", "align-equals"])
        .arg(&path)
        .assert();
    let warm = prose()
        .args(["--verbose", "check", "--select", "align-equals"])
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert();
    assert!(
        stderr_utf8(&warm).contains("1 hits, 0 misses"),
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

    std::fs::write(
        project.path().join("pyproject.toml"),
        "[tool.prose]\ncode-line-length = 100\n",
    )
    .expect("writes pyproject");
    let assert = prose()
        .args(["--verbose", "check"])
        .arg(&py)
        .current_dir(project.path())
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .success();
    let err = stderr_utf8(&assert);
    assert!(err.contains("0 hits, 1 misses"), "stderr was {err:?}");
}

#[test]
fn cache_keys_each_file_against_its_governing_config() {
    let parent = tempdir().expect("tempdir");
    let (suppressed, flagged) = sibling_projects(&parent, "alpha = 1\nb = 22\n");

    let out = assert_warm_run_matches_cold(&[&suppressed, &flagged]);

    let summary = summary_line(&out);
    assert_eq!(summary["files_visited"], 2);
    assert_eq!(summary["files_changed"], 1);
}

#[rstest]
#[case::narrow_select_after_full_set(&[], &["--select", "alphabetize"])]
#[case::ignore_after_full_set(&[], &["--ignore", "align-equals"])]
#[case::full_set_after_narrow_select(&["--select", "alphabetize"], &[])]
fn cache_misses_when_selection_changes_between_runs(
    #[case] seed_filter: &[&str],
    #[case] query_filter: &[&str],
) {
    let (_dir, path) = fixture("reselect.py", "ab = 1\nx = 2\n");
    assert_reselect_misses(seed_filter, query_filter, &path);
}

#[test]
fn check_clean_fixture_exits_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.arg("check").arg(&path).assert().success();
}

#[test]
fn check_clean_summary_anchors_with_hyacinth() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.arg("check").arg(&path).assert().success();
    let err = stderr_utf8(&assert);
    assert_eq!(err.trim(), "🪻 All clean.");
}

#[test]
fn check_dash_clean_exits_zero() {
    prose()
        .args(["check", "-"])
        .write_stdin("x = 1\n")
        .assert()
        .success();
}

#[test]
fn check_dash_unaligned_exits_format_change() {
    prose()
        .args(["check", "-"])
        .write_stdin("ab = 1\nx = 2\n")
        .assert()
        .code(1);
}

#[test]
fn check_file_in_another_project_draws_its_own_config() {
    let cwd_project = suppressed_project();
    let (_dir, path) = fixture("unaligned.py", "alpha = 1\nb = 22\n");

    prose()
        .args(["check", "--no-cache"])
        .arg(&path)
        .current_dir(cwd_project.path())
        .assert()
        .code(1);
}

#[test]
fn check_json_closes_clean_run_with_summary_envelope() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .assert()
        .success();
    let out = stdout_utf8(&assert);
    let summary = summary_line(&out);
    assert_eq!(summary["kind"], "summary");
    assert_eq!(summary["diagnostics_total"], 0);
    assert_eq!(summary["files_visited"], 1);
    assert_eq!(summary["files_changed"], 0);
}

#[test]
fn check_json_counts_a_collapsing_literal_as_changed() {
    let (_dir, path) = fixture("collapse.py", "d = {\n    \"a\": 1,\n    \"b\": 2,\n}\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .assert()
        .code(1);
    let out = stdout_utf8(&assert);
    let summary = summary_line(&out);
    assert_eq!(summary["files_changed"], 1);
}

#[test]
fn check_json_summary_counts_a_changed_file() {
    let (_dir, path) = fixture("misaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .assert()
        .code(1);
    let out = stdout_utf8(&assert);
    let summary = summary_line(&out);
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
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    prose()
        .args(["check", "--no-cache"])
        .arg(&path)
        .assert()
        .success();
}

#[test]
fn check_relative_path_resolves_its_ancestor_config() {
    let project = suppressed_project();
    write(project.path().join("unaligned.py"), "alpha = 1\nb = 22\n").expect("writes");

    prose()
        .args(["check", "--no-cache", "unaligned.py"])
        .current_dir(project.path())
        .assert()
        .success();
}

#[test]
fn check_resolves_each_files_config_from_its_own_project() {
    let parent = tempdir().expect("tempdir");
    let (suppressed, flagged) = sibling_projects(&parent, "alpha = 1\nb = 22\n");

    let assert = prose()
        .args(["check", "--no-cache", "--output-format", "json"])
        .args([&suppressed, &flagged])
        .assert()
        .code(1);

    let out = stdout_utf8(&assert);
    let diagnostics: Vec<serde_json::Value> = out
        .lines()
        .map(|line| serde_json::from_str(line).expect("parses"))
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
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[tool.prose.cache]\nenabled = false\n",
    )
    .expect("writes pyproject");
    let py = project.path().join("clean.py");
    std::fs::write(&py, "x = 1\n").expect("writes");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["--verbose", "check"])
        .arg(&py)
        .current_dir(project.path())
        .assert()
        .success();
    let err = stderr_utf8(&assert);
    assert!(err.contains("cache: bypassed"), "stderr was {err:?}");
}

#[test]
fn check_stdin_clean_exits_zero() {
    prose()
        .args(["check", "--stdin"])
        .write_stdin("x = 1\n")
        .assert()
        .success();
}

#[test]
fn check_stdin_resolves_config_from_the_cwd() {
    let project = suppressed_project();

    prose()
        .args(["check", "--stdin"])
        .write_stdin("alpha = 1\nb = 22\n")
        .current_dir(project.path())
        .assert()
        .success();
}

#[test]
fn check_stdin_unaligned_exits_format_change() {
    prose()
        .args(["check", "--stdin"])
        .write_stdin("ab = 1\nx = 2\n")
        .assert()
        .code(1);
}

#[test]
fn check_unaligned_fixture_exits_format_change() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.arg("check").arg(&path).assert().code(1);
}

#[test]
fn check_unparseable_fixture_exits_parse_error() {
    let (_dir, path) = fixture("broken.py", "def x(:");
    prose().arg("check").arg(&path).assert().code(3);
}

#[test]
fn check_validate_bypasses_a_check_populated_cache_entry() {
    let (_dir, path) = fixture("misaligned.py", "ab = 1\nx = 2\n");
    let (mut check_cmd, cache_dir) = prose_isolated();
    check_cmd.arg("check").arg(&path).assert().code(1);
    let assert = prose()
        .args(["check", "--validate", "--verbose"])
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .code(1);
    let err = stderr_utf8(&assert);
    assert!(
        err.contains("0 hits, 1 misses"),
        "validate must bypass the cache, stderr was {err:?}"
    );
}

#[test]
fn check_validate_flag_accepts_a_valid_rewrite() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(["check", "--validate"])
        .arg(&path)
        .assert()
        .code(1);
}

#[test]
fn check_violation_summary_anchors_with_bookmark() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.arg("check").arg(&path).assert().code(1);
    let err = stderr_utf8(&assert);
    assert!(
        err.contains("🔖 1 diagnostic in 1 file."),
        "stderr was {err:?}"
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
    let err = stderr_utf8(&assert);
    assert!(
        err.contains("\u{1b}[38;2;138;128;203m"),
        "stderr was {err:?}"
    );
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
fn color_arms_exit_zero(#[values("always", "never")] arm: &str) {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(["--color", arm, "check"])
        .arg(&path)
        .assert()
        .success();
}

#[test]
fn color_never_summary_stays_plain() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["--color", "never", "check"])
        .arg(&path)
        .assert()
        .success();
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

/// Each input drives a rule that net-shrinks the buffer (`collection-layout`
/// collapsing or re-laying-out a literal), the shape that overran the
/// rewritten buffer before reporting anchored to the source as written. A
/// panic in the binary would surface as exit code 101, not the format-change 1.
#[rstest]
#[case::two_entry_dict("d = {\n    \"a\": 1,\n    \"b\": 2,\n}\n")]
#[case::three_entry_list("xs = [\n    1,\n    2,\n    3,\n]\n")]
#[case::noncollapsible_call_dict(
    "config = {\n        \"alpha\": build_widget(first_argument, second_argument, third_argument),\n        \"beta\": build_gadget(fourth_argument, fifth_argument, sixth_argument),\n}\n"
)]
fn emitters_render_shrinking_literals_without_aborting(
    #[case] source: &str,
    #[values("text", "json")] format: &str,
) {
    let (_dir, path) = fixture("literal.py", source);
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(["check", "--output-format", format])
        .arg(&path)
        .assert()
        .code(1);
}

#[test]
fn format_dash_rewrites_unaligned_stdin_to_stdout() {
    prose()
        .args(["format", "-"])
        .write_stdin("ab = 1\nx = 2\n")
        .assert()
        .success()
        .stdout("ab = 1\nx  = 2\n");
}

#[test]
fn format_dash_writes_rewrite_to_stdout() {
    prose()
        .args(["format", "-"])
        .write_stdin("x = 1\n")
        .assert()
        .success()
        .stdout("x = 1\n");
}

#[test]
fn format_diff_off_tty_leaves_a_plain_patch() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["format", "--diff"]).arg(&path).assert().code(1);
    let stdout = stdout_utf8(&assert);
    assert!(stdout.contains("--- "), "patch header missing: {stdout:?}");
    assert!(
        !stdout.contains('🧵'),
        "decoration leaked off a TTY: {stdout:?}"
    );
}

#[test]
fn format_diff_renders_diff_and_leaves_file_unchanged() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["format", "--diff"]).arg(&path).assert().code(1);
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
    let after = std::fs::read_to_string(&path).expect("reads");
    assert_eq!(after, "ab = 1\nx = 2\n");
}

#[test]
fn format_diff_summary_reports_would_reformat() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["format", "--diff"]).arg(&path).assert().code(1);
    let err = stderr_utf8(&assert);
    assert!(
        err.contains("🗞️ 1 file would be reformatted."),
        "stderr was {err:?}"
    );
}

#[test]
fn format_json_renders_collapsing_literal_without_aborting() {
    let (_dir, path) = fixture("collapse.py", "d = {\n    \"a\": 1,\n    \"b\": 2,\n}\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["format", "--output-format", "json"])
        .arg(&path)
        .assert()
        .success();
    let stdout = stdout_utf8(&assert);
    assert!(
        stdout.contains("collection-layout"),
        "json missing the format diagnostic: {stdout:?}"
    );
}

#[test]
fn format_json_rewrites_over_a_check_cache_entry() {
    let (_dir, path) = fixture("misaligned.py", "ab = 1\nx = 2\n");
    let (mut check_cmd, cache_dir) = prose_isolated();
    check_cmd.arg("check").arg(&path).assert().code(1);
    let assert = prose()
        .args(["format", "--output-format", "json"])
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .success();
    let after = std::fs::read_to_string(&path).expect("reads");
    assert_ne!(after, "ab = 1\nx = 2\n");
    let stdout = stdout_utf8(&assert);
    assert!(
        stdout.contains("align-equals"),
        "json missing the diagnostic: {stdout}"
    );
}

#[test]
fn format_no_cache_flag_rewrites_when_needed() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    prose()
        .args(["format", "--no-cache"])
        .arg(&path)
        .assert()
        .success();
    let after = std::fs::read_to_string(&path).expect("reads");
    assert_ne!(after, "ab = 1\nx = 2\n");
}

#[test]
fn format_rewrite_summary_reports_reformatted() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.arg("format").arg(&path).assert().success();
    let err = stderr_utf8(&assert);
    assert!(err.contains("🗞️ Reformatted 1 file."), "stderr was {err:?}");
}

#[test]
fn format_rewrites_after_check_populated_the_cache() {
    let (_dir, path) = fixture("misaligned.py", "ab = 1\nx = 2\n");
    let (mut check_cmd, cache_dir) = prose_isolated();
    check_cmd.arg("check").arg(&path).assert().code(1);
    prose()
        .arg("format")
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .success();
    let after = std::fs::read_to_string(&path).expect("reads");
    assert_ne!(after, "ab = 1\nx = 2\n");
}

#[test]
fn format_stdin_resolves_config_from_the_cwd() {
    let project = suppressed_project();

    prose()
        .args(["format", "--stdin"])
        .write_stdin("alpha = 1\nb = 22\n")
        .current_dir(project.path())
        .assert()
        .success()
        .stdout("alpha = 1\nb = 22\n");
}

#[test]
fn format_unaligned_rewrites_and_re_check_is_clean() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut format_cmd, cache_dir) = prose_isolated();
    format_cmd.arg("format").arg(&path).assert().success();
    prose()
        .arg("check")
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .success();
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
fn quiet_check_reduces_summary_to_a_bare_count() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
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
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["--verbose", "check"])
        .arg(&path)
        .assert()
        .success();
    let err = stderr_utf8(&assert);
    assert!(err.contains("cache:"), "stderr was {err:?}");
    assert!(err.contains("files"));
}

#[test]
fn verbose_flag_with_no_cache_reports_bypassed() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let assert = prose()
        .args(["--verbose", "check", "--no-cache"])
        .arg(&path)
        .assert()
        .success();
    let err = stderr_utf8(&assert);
    assert!(err.contains("cache: bypassed"), "stderr was {err:?}");
}

#[test]
fn version_exits_clean() {
    prose().arg("--version").assert().success();
}

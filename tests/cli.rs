//! End-to-end tests against the `prose` binary, exercising
//! `cli::run` and the exit-code matrix.

use std::fs::write;
use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::{tempdir, TempDir};

fn fixture(name: &str, source: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    write(&path, source).expect("writes");
    (dir, path)
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

#[test]
fn cache_clean_subcommand_exits_zero_and_reports_count() {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", "clean"]).assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8");
    assert!(out.starts_with("removed "), "stdout was {out:?}");
    assert!(out.contains("entries"));
    assert!(out.contains("bytes"));
}

#[test]
fn cache_compact_subcommand_exits_zero_and_reports_count() {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", "compact"]).assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8");
    assert!(out.starts_with("removed "), "stdout was {out:?}");
}

#[test]
fn cache_hit_produces_identical_diagnostics_to_miss() {
    let (_dir, path) = fixture("ab.py", "ab = 1\nx = 2\n");
    let (mut miss_cmd, cache_dir) = prose_isolated();
    let miss = miss_cmd
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .assert()
        .code(1);
    let miss_stdout = miss.get_output().stdout.clone();

    let hit = prose()
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .env("PROSE_CACHE_DIR", cache_dir.path())
        .assert()
        .code(1);
    let hit_stdout = hit.get_output().stdout.clone();

    assert_eq!(miss_stdout, hit_stdout);
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
    let err = String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8");
    assert!(err.contains("0 hits, 1 misses"), "stderr was {err:?}");
}

#[test]
fn cache_info_subcommand_prints_path_and_counts() {
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["cache", "info"]).assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8");
    assert!(out.contains("path:"), "stdout was {out:?}");
    assert!(out.contains("entries: 0"));
    assert!(out.contains("bytes: 0"));
}

#[test]
fn check_clean_fixture_exits_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.arg("check").arg(&path).assert().success();
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
    let err = String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8");
    assert!(err.contains("cache: bypassed"), "stderr was {err:?}");
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
fn check_stdin_clean_exits_zero() {
    prose()
        .args(["check", "--stdin"])
        .write_stdin("x = 1\n")
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
fn color_arms_exit_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    for arm in ["always", "never"] {
        let (mut cmd, _cache_dir) = prose_isolated();
        cmd.args(["--color", arm, "check"])
            .arg(&path)
            .assert()
            .success();
    }
}

#[test]
fn completions_bash_exits_zero() {
    prose().args(["completions", "bash"]).assert().success();
}

#[test]
fn config_errors_exit_four() {
    let cases: &[&[&str]] = &[
        &["check", "--stdin", "."],
        &["check", "-", "--stdin"],
        &["check", "-", "a.py"],
        &["--not-a-flag"],
        &["check", "--select", "not-a-rule", "."],
        &["format", "--diff", "--output-format", "json", "."],
    ];
    for args in cases {
        prose().args(*args).assert().code(4);
    }
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
fn format_diff_renders_diff_and_leaves_file_unchanged() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(["format", "--diff"]).arg(&path).assert().code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8");
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
fn verbose_flag_prints_cache_telemetry_to_stderr() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd
        .args(["--verbose", "check"])
        .arg(&path)
        .assert()
        .success();
    let err = String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8");
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
    let err = String::from_utf8(assert.get_output().stderr.clone()).expect("utf-8");
    assert!(err.contains("cache: bypassed"), "stderr was {err:?}");
}

#[test]
fn version_exits_clean() {
    prose().arg("--version").assert().success();
}

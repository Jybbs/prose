//! End-to-end tests against the `prose` binary, exercising
//! `cli::run` and the exit-code matrix.

use std::fs::write;
use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::{tempdir, TempDir};

fn prose() -> Command {
    Command::cargo_bin("prose").expect("prose binary")
}

fn fixture(name: &str, source: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    write(&path, source).expect("writes");
    (dir, path)
}

#[test]
fn cache_clean_subcommand_exits_zero_and_reports_count() {
    let cache_home = tempdir().expect("tempdir");
    let assert = prose()
        .args(["cache", "clean"])
        .env("XDG_CACHE_HOME", cache_home.path())
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8");
    assert!(out.starts_with("removed "), "stdout was {out:?}");
    assert!(out.contains("entries"));
    assert!(out.contains("bytes"));
}

#[test]
fn cache_hit_produces_identical_diagnostics_to_miss() {
    let cache_home = tempdir().expect("tempdir");
    let (_dir, path) = fixture("ab.py", "ab = 1\nx = 2\n");
    let miss = prose()
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .env("XDG_CACHE_HOME", cache_home.path())
        .assert()
        .code(1);
    let miss_stdout = miss.get_output().stdout.clone();

    let hit = prose()
        .args(["check", "--output-format", "json"])
        .arg(&path)
        .env("XDG_CACHE_HOME", cache_home.path())
        .assert()
        .code(1);
    let hit_stdout = hit.get_output().stdout.clone();

    assert_eq!(miss_stdout, hit_stdout);
}

#[test]
fn check_clean_fixture_exits_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    prose().arg("check").arg(&path).assert().success();
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
    prose().arg("check").arg(&path).assert().code(1);
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
        prose()
            .args(["--color", arm, "check"])
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
        &["--not-a-flag"],
        &["check", "--select", "not-a-rule", "."],
        &["format", "--diff", "--output-format", "json", "."],
    ];
    for args in cases {
        prose().args(*args).assert().code(4);
    }
}

#[test]
fn format_diff_renders_diff_and_leaves_file_unchanged() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    let assert = prose()
        .args(["format", "--diff"])
        .arg(&path)
        .assert()
        .code(1);
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
    prose().arg("format").arg(&path).assert().success();
    prose().arg("check").arg(&path).assert().success();
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
fn version_exits_clean() {
    prose().arg("--version").assert().success();
}

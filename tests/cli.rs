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
fn check_clean_fixture_exits_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    prose().arg("check").arg(&path).assert().success();
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
fn color_always_exits_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    prose()
        .args(["--color", "always", "check"])
        .arg(&path)
        .assert()
        .success();
}

#[test]
fn color_never_exits_zero() {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    prose()
        .args(["--color", "never", "check"])
        .arg(&path)
        .assert()
        .success();
}

#[test]
fn completions_bash_exits_zero() {
    prose().args(["completions", "bash"]).assert().success();
}

#[test]
fn diff_with_json_format_exits_config_error() {
    prose()
        .args(["format", "--diff", "--output-format", "json"])
        .arg(".")
        .assert()
        .code(4);
}

#[test]
fn format_diff_renders_diff_and_leaves_file_unchanged() {
    let (_dir, path) = fixture("unaligned.py", "ab = 1\nx = 2\n");
    prose()
        .args(["format", "--diff"])
        .arg(&path)
        .assert()
        .code(1);
    let after = std::fs::read_to_string(&path).expect("reads");
    assert_eq!(after, "ab = 1\nx = 2\n");
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
fn stdin_with_positional_paths_exits_config_error() {
    prose().args(["check", "--stdin", "."]).assert().code(4);
}

#[test]
fn unknown_flag_exits_config_error() {
    prose().arg("--not-a-flag").assert().code(4);
}

#[test]
fn unknown_rule_exits_config_error() {
    prose()
        .args(["check", "--select", "not-a-rule"])
        .arg(".")
        .assert()
        .code(4);
}

#[test]
fn version_exits_clean() {
    prose().arg("--version").assert().success();
}

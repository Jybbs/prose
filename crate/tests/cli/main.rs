//! End-to-end tests against the `prose` binary, exercising
//! `cli::run` and the exit-code matrix.

mod cache;
mod check;
mod commands;
mod format;
mod global;
mod notebook;

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use assert_cmd::{Command, assert::Assert};
use tempfile::{TempDir, tempdir};

/// The shared notebook fixture whose second cell `format` aligns.
const ALIGNS: &str = include_str!("../fixtures/notebook/code_cell_aligns/input.ipynb");

/// The `check` invocation the cache tests replay, its JSON output the
/// shape a byte-for-byte comparison reads.
const CHECK_JSON: [&str; 3] = ["check", "--output-format", "json"];

/// A two-entry dict literal `reflow-collections` collapses, the shape
/// that net-shrinks the rewritten buffer.
const COLLAPSING_DICT: &str = "d = {\n    \"a\": 1,\n    \"b\": 2,\n}\n";

/// A module whose bare `import os` draws a lint the formatter leaves
/// standing, so a `format` run discloses it beside the rewrite.
const LINT_ONLY: &str = "import os\nos.getcwd()\n";

/// A `[tool.prose]` table disabling the rule the shared fixture
/// content fires, so a file governed by it checks clean.
const SUPPRESSING_PYPROJECT: &str = "[tool.prose.rules]\nalign-equals = false\n";

/// Two code cells, the second carrying a misaligned assignment pair so
/// its diagnostic ranges into a row the first cell pushes past in the
/// concatenated source, proving the report translates it cell-relative.
/// That cell also reads both modules bare, leaving the import lints
/// quiet.
static TWO_CODE_CELLS: LazyLock<String> =
    LazyLock::new(|| notebook_of(&["import os\nimport sys", "x = 1\nyyy = 2\nprint(os, sys)"]));

/// The unaligned two-assignment source the reformat and config-resolution
/// tests reuse. `AB` is `SCREAMING_CASE` and `x` a single character, so the
/// lint rules pass it silently while `align-equals` still reshapes it.
const UNALIGNED: &str = "AB = 1\nx = 2\n";

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

fn fixture(name: &str, source: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    fs_err::write(&path, source).expect("writes");
    (dir, path)
}

/// Formats `path` twice with the cache off, returning the text after
/// the first run beside the second run's assertion.
fn format_twice(path: &Path) -> (String, Assert) {
    let format = || prose().args(["format", "--no-cache"]).arg(path).assert();
    format().success();
    let once = fs_err::read_to_string(path).expect("reads");
    (once, format())
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("valid JSON")
}

/// Writes the JSON of a Python notebook holding one code cell per entry
/// of `cells`.
fn notebook_of(cells: &[&str]) -> String {
    serde_json::json!({
        "cells": cells.iter().map(|cell| serde_json::json!({
            "cell_type": "code",
            "execution_count": null,
            "metadata": {},
            "outputs": [],
            "source": [cell],
        })).collect::<Vec<_>>(),
        "metadata": {"language_info": {"name": "python"}},
        "nbformat": 4,
        "nbformat_minor": 5,
    })
    .to_string()
}

/// Creates a project directory holding `pyproject` as its `pyproject.toml`.
fn project(pyproject: &str) -> TempDir {
    let dir = tempdir().expect("tempdir");
    write_pyproject(dir.path(), pyproject);
    dir
}

fn prose() -> Command {
    Command::cargo_bin("prose").expect("prose binary")
}

fn prose_isolated() -> (Command, TempDir) {
    let dir = tempdir().expect("tempdir");
    (prose_with_cache(dir.path()), dir)
}

/// Builds a `prose` command reading and writing the cache under `dir`.
fn prose_with_cache(dir: &Path) -> Command {
    let mut cmd = prose();
    cmd.env("PROSE_CACHE_DIR", dir);
    cmd
}

/// Runs `args` against a fresh fixture holding `source`, appending the
/// fixture's path last, and returns the assertion beside the file's
/// contents after the run. The run reads a cache directory of its own,
/// so a case never inherits an entry another run left behind.
fn rewrite_fixture(name: &str, source: &str, args: &[&str]) -> (Assert, String) {
    let (_dir, path) = fixture(name, source);
    let (mut cmd, _cache_dir) = prose_isolated();
    let assert = cmd.args(args).arg(&path).assert();
    let after = fs_err::read_to_string(&path).expect("reads");
    (assert, after)
}

/// Runs `args` against a fresh fixture the way [`rewrite_fixture`] does,
/// returning the assertion alone.
fn run_fixture(name: &str, source: &str, args: &[&str]) -> Assert {
    rewrite_fixture(name, source, args).0
}

/// Runs `args` with `source` on stdin. The run reads a cache directory
/// of its own, so a case never reaches the user-level cache a developer
/// shares across projects.
fn run_stdin(source: &str, args: &[&str]) -> Assert {
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(args).write_stdin(source).assert()
}

/// Runs `args` with `source` on stdin from the working directory `cwd`,
/// under a cache directory of its own.
fn run_stdin_in(cwd: &Path, source: &str, args: &[&str]) -> Assert {
    let (mut cmd, _cache_dir) = prose_isolated();
    cmd.args(args).write_stdin(source).current_dir(cwd).assert()
}

/// Lays out two sibling projects holding identical `source`, with
/// `suppressed/x.py` under a config disabling `align-equals` and
/// `flagged/y.py` under none.
fn sibling_projects(parent: &TempDir, source: &str) -> (PathBuf, PathBuf) {
    let suppressed = parent.path().join("suppressed");
    let flagged = parent.path().join("flagged");
    fs_err::create_dir_all(&suppressed).expect("dirs create");
    fs_err::create_dir_all(&flagged).expect("dirs create");
    write_pyproject(&suppressed, SUPPRESSING_PYPROJECT);
    let x = suppressed.join("x.py");
    let y = flagged.join("y.py");
    fs_err::write(&x, source).expect("writes");
    fs_err::write(&y, source).expect("writes");
    (x, y)
}

fn stderr_utf8(assert: &Assert) -> &str {
    str::from_utf8(&assert.get_output().stderr).expect("utf-8")
}

fn stdout_utf8(assert: &Assert) -> &str {
    str::from_utf8(&assert.get_output().stdout).expect("utf-8")
}

fn summary_line(out: &str) -> serde_json::Value {
    json(out.lines().last().expect("a summary line"))
}

/// Writes the `[tool.prose]` table setting `target-version` to `version`.
fn target_version(version: &str) -> String {
    format!("[tool.prose]\ntarget-version = \"{version}\"\n")
}

/// Runs `args` with the cache off over a fresh fixture holding `source`
/// beside a `pyproject.toml` setting `target-version` to `version`,
/// returning the assertion beside the fixture's path.
fn targeting(name: &str, source: &str, version: &str, args: &[&str]) -> (Assert, PathBuf) {
    let (project, path) = fixture(name, source);
    write_pyproject(project.path(), &target_version(version));
    let assert = prose().args(args).arg("--no-cache").arg(&path).assert();
    (assert, path)
}

/// Runs `seed` against `path` under a cache directory of its own,
/// asserting it closed at `code`, and returns a command bound to that
/// same warm cache for the follow-up run.
fn warmed_by(path: &Path, seed: &[&str], code: i32) -> (Command, TempDir) {
    let (mut cmd, cache_dir) = prose_isolated();
    cmd.args(seed).arg(path).assert().code(code);
    (prose_with_cache(cache_dir.path()), cache_dir)
}

fn write_pyproject(dir: &Path, contents: &str) {
    fs_err::write(dir.join("pyproject.toml"), contents).expect("writes pyproject");
}

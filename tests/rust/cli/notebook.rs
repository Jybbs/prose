//! Tests of Jupyter notebooks across `check` and `format`.

use std::sync::LazyLock;

use rstest::rstest;

use crate::{
    ALIGNS, TWO_CODE_CELLS, assert_stderr_has, assert_stdout_has, fixture, format_twice, json,
    notebook_of, prose, rewrite_fixture, run_fixture, run_stdin, stderr_utf8, stdout_utf8,
    targeting, warmed_by,
};

/// A Python notebook whose sole code cell uses CRLF line endings. The
/// rewrite aligns the assignment while preserving each `\r\n`.
static CRLF_CELLS: LazyLock<String> = LazyLock::new(|| notebook_of(&["ab = 1\r\nx = 2\r\n"]));

/// The shared notebook fixture holding no cell `format` changes.
const EMPTY: &str = include_str!("../fixtures/notebook/empty/input.ipynb");

/// A two-cell notebook whose out-of-order imports sit in the first cell,
/// so the lone diagnostic falls in a non-last cell and the text emitter
/// renders it under that cell's header. The second cell reads both
/// modules bare, leaving the import lints quiet.
static FIRST_CELL_UNSORTED: LazyLock<String> =
    LazyLock::new(|| notebook_of(&["import sys\nimport os", "value = 1\nprint(os, sys)"]));

/// The shared notebook fixture interspersing Markdown cells among its
/// code cells.
const INTERLEAVED: &str = include_str!("../fixtures/notebook/markdown_interleaved/input.ipynb");

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

/// A two-cell notebook whose first cell closes on an unread `import json`
/// one blank row below `x = 1`, a name the module binds of its own.
static UNREAD_CLOSING_CELL: LazyLock<String> =
    LazyLock::new(|| notebook_of(&["x = 1\n\nimport json", "print(x)"]));

#[test]
fn notebook_cell_opening_on_a_future_import_passes() {
    let notebook = notebook_of(&["import os\n", "from __future__ import annotations\n"]);

    let assert = run_fixture("nb.ipynb", &notebook, &["check", "--no-cache"]);

    assert!(!stderr_utf8(&assert).contains("syntax error"));
}

#[test]
fn notebook_check_json_renders_cell_relative_with_cell_number() {
    let assert = run_fixture(
        "nb.ipynb",
        &TWO_CODE_CELLS,
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

#[rstest]
#[case::the_align_diagnostic(ALIGNS, "align-equals")]
#[case::a_diagnostic_spanning_two_cells(
    &notebook_of(&["x = 1  # a", "yy = 2  # bb", "zzz = 3  # ccc"]),
    "cells 1 to 2"
)]
#[case::a_non_last_cell_under_its_header(&FIRST_CELL_UNSORTED, "cell 1")]
#[case::a_later_cell_under_its_header(&TWO_CODE_CELLS, "cell 2")]
fn notebook_check_text_renders(#[case] notebook: &str, #[case] needle: &str) {
    let assert = run_fixture("nb.ipynb", notebook, &["check", "--no-cache"]).code(1);

    assert_stdout_has(&assert, needle);
}

#[test]
fn notebook_check_text_renders_a_cell_header_off_a_hit() {
    let (_dir, path) = fixture("nb.ipynb", &TWO_CODE_CELLS);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    let assert = warm
        .args(["--verbose", "check"])
        .arg(&path)
        .assert()
        .code(1);

    assert_stderr_has(&assert, "1 hits");
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
fn notebook_discovered_in_a_directory_walk() {
    let (dir, _notebook) = fixture("nb.ipynb", ALIGNS);

    prose()
        .args(["format", "--no-cache"])
        .arg(dir.path())
        .assert()
        .success();

    let after = fs_err::read_to_string(dir.path().join("nb.ipynb")).expect("reads");
    assert_eq!(json(&after)["cells"][1]["source"][0], "x  = 1\n");
}

#[test]
fn notebook_empty_is_a_clean_no_op() {
    run_fixture("nb.ipynb", EMPTY, &["format", "--no-cache"]).success();
}

#[rstest]
#[case::malformed_json("{not valid json")]
#[case::an_unparseable_cell(&notebook_of(&["def f("]))]
fn notebook_exits_parse_error_on(#[case] notebook: &str) {
    run_fixture("nb.ipynb", notebook, &["check", "--no-cache"]).code(3);
}

#[test]
fn notebook_format_ends_a_cell_on_the_row_above_its_dropped_import() {
    let (assert, after) =
        rewrite_fixture("nb.ipynb", &UNREAD_CLOSING_CELL, &["format", "--no-cache"]);
    assert.success();

    assert_eq!(
        json(&after)["cells"][0]["source"],
        serde_json::json!(["x = 1"])
    );
}

#[test]
fn notebook_format_is_idempotent() {
    let (_dir, path) = fixture("nb.ipynb", ALIGNS);
    let (once, second) = format_twice(&path);
    second.success();
    let twice = fs_err::read_to_string(&path).expect("reads");
    assert_eq!(once, twice);
}

#[test]
fn notebook_format_preserves_crlf_line_endings() {
    let (assert, after) = rewrite_fixture("nb.ipynb", &CRLF_CELLS, &["format", "--no-cache"]);
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
fn notebook_lazy_import_cell_passes_at_its_version() {
    let notebook = notebook_of(&["lazy import os\nprint(os.sep)\n"]);

    let (assert, _) = targeting("nb.ipynb", &notebook, "3.15", &["check"]);

    assert!(!stderr_utf8(&assert).contains("syntax error"));
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

#[rstest]
#[case::a_lazy_import_below_its_version(
    "lazy import os\nprint(os.sep)\n",
    "3.14",
    "cell 2, line 1, column 1: Cannot use `lazy` import statement"
)]
#[case::a_lazy_import_in_a_function(
    "def f():\n    lazy import os\n",
    "3.15",
    "cell 2, line 2, column 5: lazy import not allowed inside functions"
)]
fn notebook_refused_cell_names_its_cell(
    #[case] cell: &str,
    #[case] version: &str,
    #[case] position: &str,
) {
    let notebook = notebook_of(&["x = 1\n", cell]);

    let (assert, _) = targeting("nb.ipynb", &notebook, version, &["check"]);

    let assert = assert.code(3);
    assert_stderr_has(&assert, position);
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
        json(stdout_utf8(&assert))["cells"][1]["source"][0],
        "x  = 1\n"
    );
}

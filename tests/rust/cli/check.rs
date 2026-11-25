//! Tests of `prose check`, its exit codes, and its output formats.

use rstest::rstest;
use tempfile::tempdir;

use crate::{
    CHECK_JSON, COLLAPSING_DICT, SUPPRESSING_PYPROJECT, UNALIGNED, assert_stderr_has,
    assert_stdout_has, fixture, json, project, prose, prose_isolated, run_fixture, run_stdin,
    run_stdin_in, sibling_projects, stderr_utf8, stdout_utf8, summary_line, target_version,
    targeting, warmed_by, write_pyproject,
};

#[rstest]
fn check_builds_each_table_line_overflow_shares_once(
    #[values("chains", "expanded", "framed", "rewraps")] table: &str,
) {
    let (dir, path) = fixture(
        "shared.py",
        "def f():\n    \"\"\"Summary.\n\n    Body.\n    \"\"\"\n    return items.filter(a).map(b).sort()\n",
    );

    let assert = prose()
        .env("PROSE_CARRY_TRACE", "1")
        .args(["check", "--no-cache"])
        .arg(&path)
        .current_dir(dir.path())
        .assert();

    assert_eq!(
        stderr_utf8(&assert)
            .matches(&format!("build\t{table}\n"))
            .count(),
        1,
    );
}

#[test]
fn check_clean_summary_anchors_with_hyacinth() {
    let assert = run_fixture("clean.py", "x = 1\n", &["check"]).success();

    assert_eq!(stderr_utf8(&assert).trim(), "🪻 All clean.");
}

#[rstest]
#[case::a_clean_module_with_the_cache_off("clean.py", "x = 1\n", &["check", "--no-cache"], 0)]
#[case::an_unaligned_module_under_validate("unaligned.py", UNALIGNED, &["check", "--validate"], 1)]
#[case::a_module_pruning_its_unread_import(
    "mod.py",
    "import numpy as np\n\n__all__ = []\n",
    &["check", "--no-cache"],
    1
)]
#[case::a_package_init_reporting_its_unread_import(
    "__init__.py",
    "import numpy as np\n",
    &["check", "--no-cache"],
    2
)]
#[case::an_unparseable_module("broken.py", "def x(:", &["check"], 3)]
fn check_exits_the_status_its_file_draws(
    #[case] name: &str,
    #[case] source: &str,
    #[case] args: &[&str],
    #[case] code: i32,
) {
    run_fixture(name, source, args).code(code);
}

#[rstest]
#[case::clean("x = 1\n", 0)]
#[case::unaligned(UNALIGNED, 1)]
fn check_exits_the_status_its_standard_input_draws(
    #[values("-", "--stdin")] flag: &str,
    #[case] source: &str,
    #[case] code: i32,
) {
    run_stdin(source, &["check", flag]).code(code);
}

#[test]
fn check_file_in_another_project_draws_its_own_config() {
    let cwd_project = project(SUPPRESSING_PYPROJECT);
    let (_dir, path) = fixture("unaligned.py", UNALIGNED);

    prose()
        .args(["check", "--no-cache"])
        .arg(&path)
        .current_dir(cwd_project.path())
        .assert()
        .code(1);
}

#[test]
fn check_github_reports_a_refused_file_as_an_error_annotation() {
    let (assert, _) = targeting(
        "gated.py",
        "lazy import os\n",
        "3.14",
        &["check", "--output-format", "github"],
    );

    let assert = assert.code(3);
    assert_stdout_has(
        &assert,
        ",line=1,col=1,endLine=1,endColumn=5::Cannot use `lazy`",
    );
    assert_stdout_has(&assert, "::error file=");
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
fn check_json_reports_a_refused_file_as_a_syntax_error_record() {
    let (assert, path) = targeting("gated.py", "lazy import os\n", "3.14", &CHECK_JSON);

    let assert = assert.code(3);
    let records: Vec<serde_json::Value> = stdout_utf8(&assert).lines().map(json).collect();
    let record = &records[0];
    assert_eq!(record["kind"], "syntax_error");
    assert_eq!(record["filename"], path.display().to_string());
    assert_eq!(
        record["location"],
        serde_json::json!({"row": 1, "column": 1})
    );
    assert_eq!(
        record["end_location"],
        serde_json::json!({"row": 1, "column": 5})
    );
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|message| message.ends_with("under `target-version = \"3.14\"`")),
        "message was {}",
        record["message"],
    );
    assert_eq!(records[1]["kind"], "summary");
}

#[rstest]
#[case::a_misaligned_module("misaligned.py", UNALIGNED)]
#[case::a_collapsing_literal("collapse.py", COLLAPSING_DICT)]
fn check_json_summary_counts_a_changed_file(#[case] name: &str, #[case] source: &str) {
    let summary = check_json_summary(name, source, 1);

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
fn check_names_the_refusal_formatting_leaves_standing() {
    let assert = run_fixture(
        "late.py",
        "import os\nfrom __future__ import annotations\n\n\ndef f():\n    lazy import json\n    return json\n\n\nprint(os.sep)\n",
        &["check", "--no-cache"],
    )
    .code(3);

    assert_stderr_has(
        &assert,
        "line 6, column 5: lazy import not allowed inside functions",
    );
}

#[test]
fn check_passes_a_future_import_a_rule_moves_to_the_front() {
    let assert = run_fixture(
        "late.py",
        "import os\nfrom __future__ import annotations\n\nprint(os.sep)\n",
        &["check", "--no-cache"],
    );

    assert_ne!(assert.get_output().status.code(), Some(3));
    assert!(!stderr_utf8(&assert).contains("syntax error"));
}

#[rstest]
fn check_reads_the_equals_widenings_from_the_columns_table(
    #[values("align-colons", "align-match-case", "align-colons,align-match-case")] rules: &str,
) {
    let (dir, path) = fixture(
        "widened.py",
        "x: int = 1\nyy = 2\n\n\ndef f(a):\n    match a:\n        case 1:\n            return x\n        case 22:\n            return yy\n",
    );

    let assert = prose()
        .env("PROSE_CARRY_TRACE", "1")
        .args(["check", "--no-cache", "--select"])
        .arg(format!("{rules},align-equals"))
        .arg(&path)
        .current_dir(dir.path())
        .assert();

    assert_eq!(stderr_utf8(&assert).matches("build\tcolumns\n").count(), 1);
}

#[rstest]
#[case::a_stub("stub.pyi", 3)]
#[case::a_module("module.py", 0)]
fn check_refuses_a_stub_annotation_a_module_accepts(#[case] name: &str, #[case] code: i32) {
    let (assert, _) = targeting(
        name,
        "def f(x: (y := 1)) -> None: ...\n",
        "3.13",
        &["check"],
    );

    assert.code(code);
}

/// Each source holds syntax the target version rules out, or a semantic
/// syntax error Python's `compile` raises, so the run exits at the
/// parse-error status naming the file, the position, and the target.
#[rstest]
#[case::lazy_import_below_its_version(
    "lazy import os\n",
    "3.14",
    "line 1, column 1",
    "Cannot use `lazy` import statement on Python 3.14"
)]
#[case::lazy_import_in_a_class(
    "class C:\n    lazy import os\n",
    "3.15",
    "line 2, column 5",
    "lazy import not allowed inside classes"
)]
#[case::lazy_import_in_a_function(
    "def f():\n    lazy import os\n",
    "3.15",
    "line 2, column 5",
    "lazy import not allowed inside functions"
)]
#[case::lazy_import_in_a_try_block(
    "try:\n    lazy import os\nexcept ImportError:\n    pass\n",
    "3.15",
    "line 2, column 5",
    "lazy import not allowed inside try/except blocks"
)]
#[case::lazy_future_import(
    "lazy from __future__ import annotations\n\n\ndef f(x: Later) -> None:\n    pass\n\n\nclass Later:\n    pass\n",
    "3.15",
    "line 1, column 1",
    "lazy from __future__ import is not allowed"
)]
#[case::lazy_star_import(
    "lazy from os import *\n",
    "3.15",
    "line 1, column 1",
    "lazy from ... import * is not allowed"
)]
#[case::unary_plus_pattern_below_its_version(
    "match 1:\n    case +1:\n        pass\n",
    "3.14",
    "line 2, column 10",
    "Unary '+' is not allowed in a literal pattern on Python 3.14"
)]
fn check_refuses_syntax_the_target_rejects(
    #[case] source: &str,
    #[case] version: &str,
    #[case] position: &str,
    #[case] message: &str,
) {
    let (assert, path) = targeting("gated.py", source, version, &["check"]);

    let assert = assert.code(3);
    assert_stderr_has(
        &assert,
        &format!(
            "error: syntax error in `{}`, {position}: {message}",
            path.display()
        ),
    );
    assert_stderr_has(&assert, &format!("under `target-version = \"{version}\"`"));
}

#[test]
fn check_relative_path_resolves_its_ancestor_config() {
    let project = project(SUPPRESSING_PYPROJECT);
    fs_err::write(project.path().join("unaligned.py"), UNALIGNED).expect("writes");

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
        .args(CHECK_JSON)
        .arg("--no-cache")
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
    let (project, py) = fixture("clean.py", "x = 1\n");
    write_pyproject(project.path(), "[tool.prose.cache]\nenabled = false\n");
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
fn check_sarif_reports_a_refused_file_as_an_error_result() {
    let (assert, _) = targeting(
        "gated.py",
        "lazy import os\n",
        "3.14",
        &["check", "--output-format", "sarif"],
    );

    let assert = assert.code(3);
    let document = json(stdout_utf8(&assert));
    let result = &document["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "syntax-error");
    assert_eq!(result["level"], "error");
    assert_eq!(
        result["locations"][0]["physicalLocation"]["region"]["startLine"],
        1
    );
}

#[rstest]
#[case::prose_off("# prose: off\nlazy import os\n")]
#[case::fmt_off("# fmt: off\nlazy import os\n")]
fn check_skips_the_syntax_check_of_a_suppressed_file(#[case] source: &str) {
    let (assert, _) = targeting("suppressed.py", source, "3.14", &["check"]);

    assert.success();
}

#[test]
fn check_stdin_refuses_syntax_the_target_rejects() {
    let project = project(&target_version("3.14"));

    let assert = run_stdin_in(project.path(), "lazy import os\n", &["check", "--stdin"]).code(3);

    assert_stderr_has(
        &assert,
        "line 1, column 1: Cannot use `lazy` import statement",
    );
}

#[test]
fn check_stdin_resolves_config_from_the_cwd() {
    let project = project(SUPPRESSING_PYPROJECT);

    run_stdin_in(project.path(), UNALIGNED, &["check", "--stdin"]).success();
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
fn check_violation_summary_anchors_with_bookmark() {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["check"]).code(1);

    assert_stderr_has(&assert, "🔖 1 diagnostic in 1 file.");
}

#[test]
fn check_warns_a_precedence_note_once_across_both_config_loads() {
    let (project, _module) = fixture("a.py", "x = 1\n");
    fs_err::write(project.path().join("prose.toml"), "code-line-length = 90\n")
        .expect("writes prose.toml");
    write_pyproject(project.path(), "[tool.prose]\ncode-line-length = 100\n");

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

/// Runs `check` with the JSON emitter over a fresh fixture holding
/// `source`, returning the summary line the run closed with.
fn check_json_summary(name: &str, source: &str, code: i32) -> serde_json::Value {
    let assert = run_fixture(name, source, &CHECK_JSON).code(code);
    summary_line(stdout_utf8(&assert))
}

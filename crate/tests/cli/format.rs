//! Tests of `prose format`, its rewrites, its diffs, and its line endings.

use assert_cmd::assert::Assert;
use rstest::rstest;

use crate::{
    COLLAPSING_DICT, LINT_ONLY, SUPPRESSING_PYPROJECT, UNALIGNED, assert_stderr_has,
    assert_stdout_has, fixture, format_twice, project, prose, prose_isolated, rewrite_fixture,
    run_fixture, run_stdin, run_stdin_in, stderr_utf8, stdout_utf8, target_version, targeting,
    warmed_by, write_pyproject,
};

/// A module breaking every line with a lone carriage return, the third
/// ending `LineEnding` carries.
const CARRIAGE_RETURN_MODULE: &str = concat!(
    "import sys\r",
    "import os\r",
    "from pathlib import Path\r",
    "VALUES = {\"b\": 1, \"a\": 2}\r",
    "def beta() -> tuple:\r",
    "    return Path, sys, os\r",
    "def alpha() -> dict:\r",
    "    return VALUES\r",
);

/// The CRLF module as `format` rewrites it, every seam the rewrite
/// writes carrying the source's own ending.
const CRLF_FORMATTED: &str = concat!(
    "import os\r\n",
    "import sys\r\n",
    "\r\n",
    "from pathlib import Path\r\n",
    "\r\n",
    "VALUES = {\"a\": 2, \"b\": 1}\r\n",
    "\r\n",
    "\r\n",
    "def alpha() -> dict:\r\n",
    "    return VALUES\r\n",
    "\r\n",
    "\r\n",
    "def beta() -> tuple:\r\n",
    "    return Path, sys, os\r\n",
);

/// A CRLF module whose imports regroup, whose dict entries sort, and
/// whose definitions band, so the rewrite writes a separator of its own
/// at each of those seams. Every definition carries a return annotation,
/// and the run emits no lint diagnostic.
const CRLF_MODULE: &str = concat!(
    "import sys\r\n",
    "import os\r\n",
    "from pathlib import Path\r\n",
    "VALUES = {\"b\": 1, \"a\": 2}\r\n",
    "def beta() -> tuple:\r\n",
    "    return Path, sys, os\r\n",
    "def alpha() -> dict:\r\n",
    "    return VALUES\r\n",
);

/// The aligned rewrite of `ESCAPE_IN_LITERAL`, its `U+001B` intact.
const ESCAPE_ALIGNED: &str = "AB = \"X\u{1b}Y\"\nc  = 2\n";

/// An unaligned assignment pair whose first value holds a literal `U+001B`.
const ESCAPE_IN_LITERAL: &str = "AB = \"X\u{1b}Y\"\nc = 2\n";

/// A module whose first four lines break CRLF and whose remainder breaks
/// LF, the shape a partial checkout conversion leaves behind.
const MIXED_ENDING_MODULE: &str = concat!(
    "import sys\r\n",
    "import os\r\n",
    "from pathlib import Path\r\n",
    "VALUES = {\"b\": 1, \"a\": 2}\r\n",
    "def beta() -> tuple:\n",
    "    return Path, sys, os\n",
    "def alpha() -> dict:\n",
    "    return VALUES\n",
);

#[test]
fn a_duplicated_unread_import_settles_before_the_ledger_probes_it() {
    let (_dir, path) = fixture(
        "dup.py",
        "__all__ = [\"x\", \"y\"]\n\nimport os\nx = 1\nimport os\ny = 2\n",
    );
    let (mut warm, _cache) = warmed_by(&path, &["format"], 0);

    assert_eq!(
        fs_err::read_to_string(&path).expect("reads the rewrite"),
        "__all__ = [\"x\", \"y\"]\n\nx = 1\ny = 2\n",
        "one run drops the repeat and the binding its drop leaves unread",
    );

    let second = warm.arg("format").arg(&path).assert().success();
    let err = stderr_utf8(&second);
    assert!(
        !err.contains("second run"),
        "the run reading the marked bytes finds them settled, stderr was {err:?}",
    );
}

#[test]
fn carry_trace_env_reports_each_table_build_and_carry_to_stderr() {
    let (dir, path) = fixture(
        "traced.py",
        "import os\n\n__all__ = [\"f\"]\n\n\ndef f():\n    return None\n",
    );

    let assert = prose()
        .env("PROSE_CARRY_TRACE", "1")
        .args(["format", "--diff", "--no-cache"])
        .arg(&path)
        .current_dir(dir.path())
        .assert();

    assert_stderr_has(&assert, "build\tbindings\n");
    assert_stderr_has(&assert, "carry\tprune-inert-imports\tbindings\tdeclined\n");
    assert_stderr_has(&assert, "second\tnarrowed\tprune-inert-imports\n");
}

#[test]
fn format_dash_exits_clean_with_lint_and_prints_no_summary_off_a_terminal() {
    let canonical = "import os\n\nos.getcwd()\n";

    run_stdin(canonical, &["format", "-"])
        .success()
        .stdout(canonical)
        .stderr("");
}

#[rstest]
#[case::a_canonical_source("x = 1\n", "x = 1\n")]
#[case::an_escape_in_a_literal(ESCAPE_IN_LITERAL, ESCAPE_ALIGNED)]
#[case::an_unaligned_source(UNALIGNED, "AB = 1\nx  = 2\n")]
fn format_dash_prints_the_rewrite_to_stdout(#[case] source: &str, #[case] rewrite: &'static str) {
    run_stdin(source, &["format", "-"])
        .success()
        .stdout(rewrite);
}

#[test]
fn format_diff_keeps_escape_bytes_in_a_plain_patch() {
    let assert = run_fixture("escape.py", ESCAPE_IN_LITERAL, &["format", "--diff"]).code(1);

    assert_patch_keeps_escape(&assert);
}

#[test]
fn format_diff_renders_a_plain_patch_and_leaves_file_unchanged() {
    let (assert, after) = rewrite_fixture("unaligned.py", UNALIGNED, &["format", "--diff"]);
    let assert = assert.code(1);

    let stdout = stdout_utf8(&assert);
    assert!(stdout.contains("--- "), "diff missing header: {stdout:?}");
    assert!(
        !stdout.contains('🧵'),
        "decoration leaked off a TTY: {stdout:?}"
    );
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
    assert_stderr_has(&assert, "🗞️ 1 file would be reformatted.");
}

#[test]
fn format_diff_rewrites_a_unary_plus_pattern_at_its_version() {
    let (assert, _) = targeting(
        "plus.py",
        "match 1:\n    case +1:\n        pass\n",
        "3.15",
        &["format", "--diff"],
    );

    let assert = assert.code(1);
    assert_stdout_has(&assert, "+    case +1: pass");
    assert!(!stderr_utf8(&assert).contains("syntax error"));
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

    let after = fs_err::read_to_string(&path).expect("reads");
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
fn format_refuses_a_file_and_leaves_it_as_written() {
    let source = "def f():\n    lazy import os\n    return os.sep\n";
    let (project, path) = fixture("gated.py", source);
    write_pyproject(project.path(), &target_version("3.15"));

    prose()
        .args(["format", "--no-cache"])
        .arg(&path)
        .assert()
        .code(3);

    assert_eq!(fs_err::read_to_string(&path).expect("reads"), source);
}

#[test]
fn format_rewrites_after_check_populated_the_cache() {
    let (_dir, path) = fixture("misaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["check"], 1);

    warm.arg("format").arg(&path).assert().success();

    let after = fs_err::read_to_string(&path).expect("reads");
    assert_ne!(after, UNALIGNED);
}

#[test]
fn format_rewrites_the_named_files_beside_a_rejected_one() {
    let (dir, path) = fixture("misaligned.py", UNALIGNED);
    let script = dir.path().join("script");
    fs_err::write(&script, UNALIGNED).expect("writes");
    let (mut cmd, _cache_dir) = prose_isolated();

    cmd.arg("format").arg(&script).arg(&path).assert().code(4);

    assert_ne!(fs_err::read_to_string(&path).expect("reads"), UNALIGNED);
    assert_eq!(fs_err::read_to_string(&script).expect("reads"), UNALIGNED);
}

#[test]
fn format_stdin_diff_keeps_escape_bytes_in_a_plain_patch() {
    let assert = run_stdin(ESCAPE_IN_LITERAL, &["format", "--stdin", "--diff"]).code(1);

    assert_patch_keeps_escape(&assert);
}

#[test]
fn format_stdin_json_exits_format_change_for_a_pending_rewrite() {
    run_stdin(UNALIGNED, &["format", "--output-format", "json", "-"]).code(1);
}

#[test]
fn format_stdin_refuses_syntax_the_target_rejects() {
    let project = project(&target_version("3.14"));

    run_stdin_in(project.path(), "lazy import os\n", &["format", "-"])
        .code(3)
        .stdout("");
}

#[test]
fn format_stdin_resolves_config_from_the_cwd() {
    let project = project(SUPPRESSING_PYPROJECT);

    run_stdin_in(project.path(), UNALIGNED, &["format", "--stdin"])
        .success()
        .stdout(UNALIGNED);
}

#[test]
fn format_stdin_under_a_broken_named_config_exits_config_error() {
    let project = project("[this is not valid TOML");
    let (mut cmd, _cache_dir) = prose_isolated();

    cmd.args(["format", "--stdin-filename"])
        .arg(project.path().join("x.py"))
        .arg("-")
        .write_stdin(UNALIGNED)
        .assert()
        .code(4)
        .stdout("");
}

#[test]
fn format_unaligned_rewrites_and_re_check_is_clean() {
    let (_dir, path) = fixture("unaligned.py", UNALIGNED);
    let (mut warm, _cache) = warmed_by(&path, &["format"], 0);

    warm.arg("check").arg(&path).assert().success();
}

#[test]
fn format_warns_an_unknown_key_once_across_both_config_loads() {
    let (project, py) = fixture("a.py", UNALIGNED);
    write_pyproject(project.path(), "[tool.prose]\nmax-shft = 4\n");

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
fn module_format_holds_each_line_ending_of_a_mixed_source() {
    let (_dir, path) = fixture("mod.py", MIXED_ENDING_MODULE);
    let (once, second) = format_twice(&path);

    assert_eq!(
        once,
        concat!(
            "import os\r\n",
            "import sys\r\n",
            "\r\n",
            "from pathlib import Path\r\n",
            "\r\n",
            "VALUES = {\"a\": 2, \"b\": 1}\r\n",
            "\r\n",
            "\r\n",
            "def alpha() -> dict:\n",
            "    return VALUES\r\n",
            "\r\n",
            "\r\n",
            "def beta() -> tuple:\n",
            "    return Path, sys, os\n",
        ),
        "a break prose writes takes the first ending found, and a line it carries keeps its own",
    );

    second.success();
    assert_eq!(once, fs_err::read_to_string(&path).expect("reads"));
}

#[rstest]
#[case::crlf(CRLF_MODULE, CRLF_FORMATTED.to_owned())]
#[case::lone_carriage_returns(CARRIAGE_RETURN_MODULE, CRLF_FORMATTED.replace("\r\n", "\r"))]
fn module_format_preserves_its_line_endings(#[case] source: &str, #[case] rewrite: String) {
    let (assert, after) = rewrite_fixture("mod.py", source, &["format", "--no-cache"]);
    assert.success();

    assert_eq!(after, rewrite);
}

#[test]
fn module_format_settles_on_crlf_input() {
    let (_dir, path) = fixture("mod.py", CRLF_MODULE);
    let (once, second) = format_twice(&path);

    let second = second.success();
    assert_eq!(once, fs_err::read_to_string(&path).expect("reads"));
    assert!(
        !stderr_utf8(&second).contains("second run"),
        "a CRLF module settles without an unstable-output report",
    );
}

fn assert_patch_keeps_escape(assert: &Assert) {
    assert_stdout_has(assert, "AB = \"X\u{1b}Y\"");
}

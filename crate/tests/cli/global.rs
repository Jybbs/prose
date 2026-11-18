//! Tests of the options and exit codes every subcommand shares.

use rstest::rstest;

use crate::{
    UNALIGNED, assert_stderr_has, assert_stdout_has, fixture, prose, prose_isolated, run_fixture,
    stderr_utf8, stdout_utf8, write_pyproject,
};

#[rstest]
#[case::truecolor(Some("truecolor"), "\u{1b}[38;2;138;128;203m")]
#[case::ansi_fallback(None, "\u{1b}[35m")]
fn color_always_summary_paints_in_the_palette_colorterm_names(
    #[case] colorterm: Option<&str>,
    #[case] anchor: &str,
) {
    let (_dir, path) = fixture("clean.py", "x = 1\n");
    let (mut cmd, _cache_dir) = prose_isolated();
    match colorterm {
        Some(value) => cmd.env("COLORTERM", value),
        None => cmd.env_remove("COLORTERM"),
    };

    let assert = cmd
        .args(["--color", "always", "check"])
        .arg(&path)
        .assert()
        .success();

    let err = stderr_utf8(&assert);
    assert!(err.contains(anchor), "stderr was {err:?}");
    assert_eq!(
        err.contains("38;2;"),
        colorterm.is_some(),
        "stderr was {err:?}"
    );
}

#[rstest]
#[case::always("always", true)]
#[case::never("never", false)]
fn color_arm_paints_the_diagnostic_body_and_summary(#[case] arm: &str, #[case] painted: bool) {
    let assert = run_fixture("unaligned.py", UNALIGNED, &["--color", arm, "check"]).code(1);

    let out = stdout_utf8(&assert);
    assert_eq!(out.contains('\u{1b}'), painted, "stdout was {out:?}");
    let err = stderr_utf8(&assert);
    assert_eq!(err.contains('\u{1b}'), painted, "stderr was {err:?}");
}

#[rstest]
#[case(&["check", "--stdin", "."])]
#[case(&["check", "-", "--stdin"])]
#[case(&["check", "-", "a.py"])]
#[case(&["check"])]
#[case(&["format"])]
#[case(&["--not-a-flag"])]
#[case(&["check", "--select", "not-a-rule", "."])]
#[case(&["format", "--diff", "--output-format", "json", "."])]
fn config_errors_exit_four(#[case] args: &[&str]) {
    prose().args(args).assert().code(4);
}

#[rstest]
fn cwd_config_error_exits_four(#[values("check", "format")] subcommand: &str) {
    let (project, _module) = fixture("a.py", "x = 1\n");
    write_pyproject(project.path(), "[this is not valid TOML\n");

    prose()
        .args([subcommand, "--no-cache", "a.py"])
        .current_dir(project.path())
        .assert()
        .code(4);
}

#[test]
fn help_exits_clean() {
    let assert = prose().arg("--help").assert().success();
    insta::assert_snapshot!(stdout_utf8(&assert));
}

#[rstest]
#[case::check(&["check"], "check")]
#[case::format(&["format"], "format")]
#[case::format_diff(&["format", "--diff"], "format")]
fn named_file_outside_the_python_extensions_exits_four(
    #[case] args: &[&str],
    #[case] subcommand: &str,
) {
    let (_dir, script) = fixture("script", UNALIGNED);
    let (mut cmd, _cache_dir) = prose_isolated();

    let assert = cmd.args(args).arg(&script).assert().code(4);

    assert_stderr_has(
        &assert,
        &format!(
            "error: cannot {subcommand} `{}`, which is not a `.py`, `.pyi`, `.pyw`, or `.ipynb` file\n",
            script.display(),
        ),
    );
    assert!(
        !stderr_utf8(&assert).contains("All clean"),
        "a clean line leaked"
    );
}

#[test]
fn no_args_prints_help_and_exits_clean() {
    let assert = prose().assert().success();
    insta::assert_snapshot!(stderr_utf8(&assert));
}

#[test]
fn version_prints_the_crate_version() {
    let assert = prose().arg("--version").assert().success();
    assert_stdout_has(&assert, env!("CARGO_PKG_VERSION"));
}

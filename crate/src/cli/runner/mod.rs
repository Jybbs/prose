//! Pipeline orchestration: load source, run, emit diagnostics, classify outcome.

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use ruff_python_ast::PySourceType;
use ruff_source_file::SourceFile;

use super::{
    args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter},
    exit_status::ExitStatus,
    output::Presentation,
};
use crate::{
    cache::{Cache, Rewrite},
    config::Config,
    diagnostics::{Diagnostic, Severity},
    pipeline::Pipeline,
};

mod diff;
mod notebook;
mod process;
mod report;
mod resolve;

use diff::write_rewrite_diff;
use process::{apply_rewrite, process_path, process_paths, process_stdin, read_stdin};
use report::{
    emit_outcomes, emitter_summary, finish, render_summary, status_from_outcomes, summarize,
};
use resolve::{ConfigResolver, Resolved};

/// One file's contribution to the run.
#[derive(Debug)]
enum FileOutcome {
    Done {
        cached: bool,
        diagnostics: Vec<Diagnostic>,
        file: SourceFile,
        rewrite: Rewrite,
    },
    Failed(ExitStatus),
}

/// Which closing summary an outcome set resolves into.
#[derive(Clone, Copy)]
enum Mode {
    Check,
    Preview,
    Reformat,
}

/// Which pipeline passes a CLI mode reads from a file.
#[derive(Clone, Copy, Debug)]
enum Pass {
    /// `format` with a `json`, `sarif`, or `github` output format: the
    /// as-written diagnostics and the rewrite.
    Both,
    /// `check`: the as-written diagnostics, no rewrite. `validate` adds
    /// the opt-in reparse guard that fails on an unparseable rule output.
    Diagnose { validate: bool },
    /// Plain `format` and `--diff`: the rewrite alone.
    Rewrite,
}

/// Per-run setup shared across every path the walker yields.
struct RunSetup {
    cache: Option<Cache>,
    cwd: Arc<Resolved>,
    resolver: ConfigResolver,
}

pub(crate) fn check_with_io<R: Read, O: Write, E: Write>(
    args: CheckArgs,
    verbose: bool,
    present: &Presentation,
    stdin: R,
    mut stdout: O,
    mut stderr: E,
) -> anyhow::Result<ExitStatus> {
    let setup = match build_run(args.rules, args.no_cache) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    let pass = Pass::Diagnose {
        validate: args.validate,
    };
    let outcomes = if args.stdin {
        let source_type = stdin_source_type(args.stdin_filename.as_deref());
        let outcome = match read_stdin(stdin) {
            Ok(text) => process_stdin(text, source_type, &setup.cwd.pipeline, pass),
            Err(outcome) => outcome,
        };
        vec![outcome]
    } else {
        process_paths(&args.paths, |path, source_type| {
            process_path(path, source_type, &setup, pass)
        })
    };
    let summary = emitter_summary(&outcomes);
    emit_outcomes(&outcomes, args.output_format, &mut stdout, &summary)?;
    let status = finish(&outcomes, setup.cache.is_some(), verbose, false);
    render_summary(
        &mut stderr,
        present,
        summarize(&outcomes, &summary, Mode::Check),
    );
    Ok(status)
}

pub(crate) fn format_with_io<R: Read, O: Write, E: Write>(
    args: FormatArgs,
    verbose: bool,
    present: &Presentation,
    stdin: R,
    mut stdout: O,
    mut stderr: E,
) -> anyhow::Result<ExitStatus> {
    let setup = match build_run(args.rules, args.no_cache) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    if args.stdin {
        let source_type = stdin_source_type(args.stdin_filename.as_deref());
        return format_stdin(
            read_stdin(stdin).map(|text| (text, source_type)),
            args.diff,
            args.output_format,
            present,
            &setup.cwd.pipeline,
            &mut stdout,
            &mut stderr,
        );
    }
    if args.diff {
        format_paths_diff(
            &args.paths,
            &setup,
            verbose,
            present,
            &mut stdout,
            &mut stderr,
        )
    } else {
        format_paths_rewrite(
            &args.paths,
            args.output_format,
            &setup,
            verbose,
            present,
            &mut stdout,
            &mut stderr,
        )
    }
}

/// Builds the run-level setup. The cwd's own config governs stdin input
/// and the cache settings, while each path input re-resolves its own
/// effective config through the resolver.
fn build_run(rules: RuleFilter, no_cache: bool) -> Result<RunSetup, ExitStatus> {
    let config = super::load_config_or_status()?;
    let cache = open_cache(&config, no_cache);
    let resolver = ConfigResolver::new(rules.select, rules.ignore);
    let cwd = resolver.seed(&config);
    Ok(RunSetup {
        cache,
        cwd,
        resolver,
    })
}

/// Resolves which pipeline passes a `format` invocation reads. Plain
/// text rewrites and `--diff` need `run` alone, whereas a `json`,
/// `sarif`, or `github` output format also renders the as-written
/// diagnostics.
fn format_pass(diff: bool, format: OutputFormat) -> Pass {
    if diff || format.is_text() {
        Pass::Rewrite
    } else {
        Pass::Both
    }
}

fn format_paths_diff<O: Write, E: Write>(
    paths: &[PathBuf],
    setup: &RunSetup,
    verbose: bool,
    present: &Presentation,
    stdout: &mut O,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let outcomes = process_paths(paths, |path, source_type| {
        process_path(path, source_type, setup, Pass::Rewrite)
    });
    for outcome in &outcomes {
        if let FileOutcome::Done {
            file,
            rewrite: Rewrite::Changed(kind),
            ..
        } = outcome
        {
            write_rewrite_diff(
                stdout,
                file.name(),
                file.source_text(),
                kind,
                present.decorate_diff(),
            )?;
        }
    }
    let summary = emitter_summary(&outcomes);
    let status = finish(&outcomes, setup.cache.is_some(), verbose, false);
    render_summary(
        stderr,
        present,
        summarize(&outcomes, &summary, Mode::Preview),
    );
    Ok(status)
}

fn format_paths_rewrite<O: Write, E: Write>(
    paths: &[PathBuf],
    format: OutputFormat,
    setup: &RunSetup,
    verbose: bool,
    present: &Presentation,
    stdout: &mut O,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let pass = format_pass(false, format);
    let outcomes = process_paths(paths, |path, source_type| {
        apply_rewrite(path, process_path(path, source_type, setup, pass))
    });
    let summary = emitter_summary(&outcomes);
    if !format.is_text() {
        emit_outcomes(&outcomes, format, stdout, &summary)?;
    }
    let status = finish(&outcomes, setup.cache.is_some(), verbose, true);
    render_summary(
        stderr,
        present,
        summarize(&outcomes, &summary, Mode::Reformat),
    );
    Ok(status)
}

fn format_stdin<O: Write, E: Write>(
    input: Result<(String, PySourceType), FileOutcome>,
    diff: bool,
    format: OutputFormat,
    present: &Presentation,
    pipeline: &Pipeline,
    writer: &mut O,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let (outcome, original) = match input {
        Ok((text, source_type)) => (
            process_stdin(
                text.clone(),
                source_type,
                pipeline,
                format_pass(diff, format),
            ),
            text,
        ),
        Err(outcome) => (outcome, String::new()),
    };
    let outcomes = std::slice::from_ref(&outcome);
    let summary = emitter_summary(outcomes);
    if let FileOutcome::Done { rewrite, .. } = &outcome {
        if diff {
            if let Rewrite::Changed(kind) = rewrite {
                write_rewrite_diff(writer, "<stdin>", &original, kind, present.decorate_diff())?;
            }
        } else if format.is_text() {
            let to_write: &[u8] = match rewrite {
                Rewrite::Changed(kind) => kind.written().as_bytes(),
                // A non-Python notebook skips the rewrite, so echo stdin verbatim.
                Rewrite::Skipped | Rewrite::Unchanged => original.as_bytes(),
            };
            writer.write_all(to_write).context("writing stdout")?;
        } else {
            emit_outcomes(outcomes, format, writer, &summary)?;
        }
    }
    let mode = if diff { Mode::Preview } else { Mode::Reformat };
    let status = status_from_outcomes(outcomes, !diff);
    render_summary(stderr, present, summarize(outcomes, &summary, mode));
    Ok(status)
}

fn has_format_change(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Format)
}

/// Resolves the source type of stdin input from a `--stdin-filename`,
/// defaulting to Python when none is given.
fn stdin_source_type(filename: Option<&Path>) -> PySourceType {
    filename
        .and_then(PySourceType::try_from_path)
        .unwrap_or_default()
}

fn open_cache(config: &Config, no_cache: bool) -> Option<Cache> {
    if no_cache || !config.cache.enabled {
        return None;
    }
    Cache::open()
        .map(|c| c.with_max_size_mib(config.cache.max_size_mib))
        .inspect_err(|e| eprintln!("warning: cache disabled: {e}"))
        .ok()
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use assert_matches::assert_matches;
    use tempfile::TempDir;

    use super::*;
    use crate::rule::RuleId;
    use crate::testing::write_pyproject;

    struct ErrorReader;

    impl Read for ErrorReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated stdin failure"))
        }
    }

    fn check_args(paths: Vec<PathBuf>, stdin: bool) -> CheckArgs {
        CheckArgs {
            no_cache: true,
            paths,
            stdin,
            ..Default::default()
        }
    }

    fn fixture(source: &str) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, source).expect("writes");
        (tmp, file)
    }

    fn format_args(paths: Vec<PathBuf>, stdin: bool, diff: bool) -> FormatArgs {
        FormatArgs {
            diff,
            no_cache: true,
            paths,
            stdin,
            ..Default::default()
        }
    }

    fn run_check(args: CheckArgs) -> ExitStatus {
        check_with_io(
            args,
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow")
    }

    fn run_format(args: FormatArgs) -> ExitStatus {
        format_with_io(
            args,
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow")
    }

    fn windowed() -> Presentation {
        Presentation {
            quiet: false,
            stdout_tty: false,
        }
    }

    #[test]
    fn check_broken_ancestor_config_fails_the_file_and_continues() {
        let tmp = TempDir::new().expect("tempdir");
        let broken = tmp.path().join("broken");
        let plain = tmp.path().join("plain");
        std::fs::create_dir_all(&broken).expect("dirs create");
        std::fs::create_dir_all(&plain).expect("dirs create");
        write_pyproject(&broken, "[this is not valid TOML");
        std::fs::write(broken.join("a.py"), "x = 1\n").expect("writes");
        std::fs::write(plain.join("b.py"), "x = 1\n").expect("writes");

        let mut args = check_args(vec![broken.join("a.py"), plain.join("b.py")], false);
        args.output_format = OutputFormat::Json;
        let mut stdout = Vec::new();
        let status = check_with_io(
            args,
            false,
            &windowed(),
            io::empty(),
            &mut stdout,
            io::sink(),
        )
        .expect("runs without anyhow");

        assert_eq!(status, ExitStatus::ConfigError);
        let out = String::from_utf8(stdout).expect("utf-8");
        let summary: serde_json::Value =
            serde_json::from_str(out.lines().last().expect("a summary line")).expect("parses");
        assert_eq!(summary["files_visited"], 1);
    }

    #[test]
    fn check_clean_returns_clean() {
        let (tmp, _file) = fixture("x = 1\n");

        let status = run_check(check_args(vec![tmp.path().to_path_buf()], false));

        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn check_ignore_overrides_the_files_own_config() {
        let (_tmp, file) = fixture("alpha = 1\nb = 22\n");

        let mut args = check_args(vec![file], false);
        args.rules.ignore = vec![RuleId::from("align-equals")];

        assert_eq!(run_check(args), ExitStatus::Clean);
    }

    #[test]
    fn check_pending_format_returns_format_change() {
        let (tmp, _file) = fixture("alpha = 1\nb = 22\n");

        let status = run_check(check_args(vec![tmp.path().to_path_buf()], false));

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn check_resolves_config_from_the_files_own_ancestors() {
        let (tmp, file) = fixture("alpha = 1\nb = 22\n");
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        assert_eq!(run_check(check_args(vec![file], false)), ExitStatus::Clean);
    }

    #[test]
    fn check_select_overrides_the_files_own_config() {
        let (tmp, file) = fixture("alpha = 1\nb = 22\n");
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        let mut args = check_args(vec![file], false);
        args.rules.select = vec![RuleId::from("align-equals")];

        assert_eq!(run_check(args), ExitStatus::FormatChange);
    }

    #[test]
    fn check_stdin_returns_clean_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let status = check_with_io(
            check_args(Vec::new(), true),
            false,
            &windowed(),
            stdin,
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");
        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn check_stdin_with_read_failure_returns_config_error() {
        let status = check_with_io(
            check_args(Vec::new(), true),
            false,
            &windowed(),
            ErrorReader,
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow");
        assert_eq!(status, ExitStatus::ConfigError);
    }

    #[test]
    fn check_unparseable_path_returns_parse_error() {
        let (tmp, _file) = fixture("def foo(");

        let status = run_check(check_args(vec![tmp.path().to_path_buf()], false));

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn check_unparseable_stdin_returns_parse_error() {
        let stdin = Cursor::new(b"def foo(".to_vec());
        let status = check_with_io(
            check_args(Vec::new(), true),
            false,
            &windowed(),
            stdin,
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow");
        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn format_diff_resolves_config_from_the_files_own_ancestors() {
        let (tmp, file) = fixture("alpha = 1\nb = 22\n");
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        let status = run_format(format_args(vec![file], false, true));

        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn format_diff_returns_clean_for_already_canonical_file() {
        let (tmp, _file) = fixture("x = 1\n");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, true));

        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn format_diff_returns_config_error_for_missing_path() {
        let tmp = TempDir::new().expect("tempdir");
        let missing = tmp.path().join("does_not_exist");

        let status = run_format(format_args(vec![missing], false, true));

        assert_eq!(status, ExitStatus::ConfigError);
    }

    #[test]
    fn format_diff_returns_format_change_for_pending_change() {
        let (tmp, _file) = fixture("alpha = 1\nb = 22\n");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, true));

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn format_pass_needs_both_only_for_a_diagnostic_format() {
        assert_matches!(format_pass(false, OutputFormat::Json), Pass::Both);
        assert_matches!(format_pass(false, OutputFormat::Text), Pass::Rewrite);
        assert_matches!(format_pass(true, OutputFormat::Json), Pass::Rewrite);
    }

    #[test]
    fn format_paths_does_not_rewrite_when_pipeline_is_empty() {
        let (tmp, file) = fixture("x = 1\n");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::Clean);
        let contents = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(contents, "x = 1\n");
    }

    #[test]
    fn format_paths_rewrite_emits_json_when_format_is_non_text() {
        let (tmp, _file) = fixture("alpha = 1\nb = 22\n");

        let mut args = format_args(vec![tmp.path().to_path_buf()], false, false);
        args.output_format = OutputFormat::Json;
        let mut stdout = Vec::new();
        let status = format_with_io(
            args,
            false,
            &windowed(),
            io::empty(),
            &mut stdout,
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
        assert!(!stdout.is_empty());
    }

    #[test]
    fn format_rewrite_resolves_config_from_the_files_own_ancestors() {
        let (tmp, file) = fixture("alpha = 1\nb = 22\n");
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        let status = run_format(format_args(vec![file.clone()], false, false));

        assert_eq!(status, ExitStatus::Clean);
        let after = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(after, "alpha = 1\nb = 22\n");
    }

    #[test]
    fn format_stdin_diff_writes_unified_hunks() {
        let stdin = Cursor::new(b"alpha = 1\nb = 22\n".to_vec());
        let mut stdout = Vec::new();
        let status = format_with_io(
            format_args(Vec::new(), true, true),
            false,
            &windowed(),
            stdin,
            &mut stdout,
            io::sink(),
        )
        .expect("runs successfully");
        assert_eq!(status, ExitStatus::FormatChange);
        let out = String::from_utf8(stdout).expect("utf-8");
        assert!(out.contains("@@"));
    }

    #[test]
    fn format_stdin_emits_json_when_format_is_non_text() {
        let stdin = Cursor::new(b"alpha = 1\nb = 22\n".to_vec());
        let mut stdout = Vec::new();
        let mut args = format_args(Vec::new(), true, false);
        args.output_format = OutputFormat::Json;
        let status = format_with_io(args, false, &windowed(), stdin, &mut stdout, io::sink())
            .expect("runs successfully");
        assert_eq!(status, ExitStatus::Clean);
        assert!(!stdout.is_empty());
    }

    #[test]
    fn format_stdin_prints_input_verbatim_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let mut stdout = Vec::new();
        let status = format_with_io(
            format_args(Vec::new(), true, false),
            false,
            &windowed(),
            stdin,
            &mut stdout,
            io::sink(),
        )
        .expect("runs successfully");
        assert_eq!(status, ExitStatus::Clean);
        assert_eq!(stdout, b"x = 1\n");
    }

    #[test]
    fn format_stdin_with_read_failure_returns_config_error() {
        let mut stdout = Vec::new();
        let status = format_with_io(
            format_args(Vec::new(), true, false),
            false,
            &windowed(),
            ErrorReader,
            &mut stdout,
            io::sink(),
        )
        .expect("runs without anyhow");
        assert_eq!(status, ExitStatus::ConfigError);
    }

    #[test]
    fn format_unparseable_returns_parse_error() {
        let (tmp, _file) = fixture("def foo(");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn format_writes_and_returns_clean_for_pending_change() {
        let (tmp, file) = fixture("alpha = 1\nb = 22\n");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::Clean);
        let after = std::fs::read_to_string(&file).expect("reads");
        assert_ne!(after, "alpha = 1\nb = 22\n");
    }

    #[test]
    fn format_writes_return_config_error_when_target_is_readonly() {
        let (tmp, file) = fixture("alpha = 1\nb = 22\n");
        let mut perms = std::fs::metadata(&file).expect("metadata").permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&file, perms).expect("set_permissions");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::ConfigError);
    }
}

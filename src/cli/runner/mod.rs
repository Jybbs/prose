//! Pipeline orchestration: load source, run, emit diagnostics, classify outcome.

use std::{
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Context;
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
mod process;
mod report;

use diff::write_diff;
use process::{apply_rewrite, process_path, process_paths, process_stdin};
use report::{
    emit_outcomes, emitter_summary, finish, render_summary, status_from_outcomes, summarize,
};

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
    config_toml: String,
    pipeline: Pipeline,
}

pub(crate) fn check_with_io<R: Read, O: Write, E: Write>(
    args: CheckArgs,
    verbose: bool,
    present: &Presentation,
    stdin: R,
    mut stdout: O,
    mut stderr: E,
) -> anyhow::Result<ExitStatus> {
    let setup = match build_run(&args.rules, args.no_cache) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    let pass = Pass::Diagnose {
        validate: args.validate,
    };
    let outcomes = if args.stdin {
        vec![process_stdin(stdin, &setup.pipeline, pass)]
    } else {
        process_paths(&args.paths, |path| process_path(path, &setup, pass))
    };
    let summary = emitter_summary(&outcomes);
    emit_outcomes(&outcomes, args.output_format, &mut stdout, &summary)?;
    let status = finish(&outcomes, &setup, verbose, false);
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
    let setup = match build_run(&args.rules, args.no_cache) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    if args.stdin {
        return format_stdin(
            stdin,
            args.diff,
            args.output_format,
            present,
            &setup.pipeline,
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

fn build_run(rules: &RuleFilter, no_cache: bool) -> Result<RunSetup, ExitStatus> {
    let config = super::load_config_or_status()?;
    let pipeline = Pipeline::with_filters(&config, &rules.select, &rules.ignore);
    let cache = open_cache(&config, no_cache);
    let config_toml = toml::to_string(&config).unwrap_or_default();
    Ok(RunSetup {
        cache,
        config_toml,
        pipeline,
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
    let outcomes = process_paths(paths, |path| process_path(path, setup, Pass::Rewrite));
    for outcome in &outcomes {
        if let FileOutcome::Done {
            file,
            rewrite: Rewrite::Changed(formatted),
            ..
        } = outcome
        {
            write_diff(
                stdout,
                file.name(),
                file.source_text(),
                formatted,
                present.decorate_diff(),
            )?;
        }
    }
    let summary = emitter_summary(&outcomes);
    let status = finish(&outcomes, setup, verbose, false);
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
    let outcomes = process_paths(paths, |path| {
        apply_rewrite(path, process_path(path, setup, pass))
    });
    let summary = emitter_summary(&outcomes);
    if !format.is_text() {
        emit_outcomes(&outcomes, format, stdout, &summary)?;
    }
    let status = finish(&outcomes, setup, verbose, true);
    render_summary(
        stderr,
        present,
        summarize(&outcomes, &summary, Mode::Reformat),
    );
    Ok(status)
}

fn format_stdin<R: Read, O: Write, E: Write>(
    stdin: R,
    diff: bool,
    format: OutputFormat,
    present: &Presentation,
    pipeline: &Pipeline,
    writer: &mut O,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let outcome = process_stdin(stdin, pipeline, format_pass(diff, format));
    let outcomes = std::slice::from_ref(&outcome);
    let summary = emitter_summary(outcomes);
    if let FileOutcome::Done { file, rewrite, .. } = &outcome {
        if diff {
            if let Rewrite::Changed(formatted) = rewrite {
                write_diff(
                    writer,
                    "<stdin>",
                    file.source_text(),
                    formatted,
                    present.decorate_diff(),
                )?;
            }
        } else if format.is_text() {
            let to_write = match rewrite {
                Rewrite::Changed(formatted) => formatted.as_str(),
                Rewrite::Skipped => unreachable!("format passes compute the rewrite"),
                Rewrite::Unchanged => file.source_text(),
            };
            writer
                .write_all(to_write.as_bytes())
                .context("writing stdout")?;
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

    fn format_args(paths: Vec<PathBuf>, stdin: bool, diff: bool) -> FormatArgs {
        FormatArgs {
            diff,
            no_cache: true,
            paths,
            stdin,
            ..Default::default()
        }
    }

    fn windowed() -> Presentation {
        Presentation {
            quiet: false,
            stdout_tty: false,
        }
    }

    #[test]
    fn check_clean_returns_clean() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let status = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn check_pending_format_returns_format_change() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "alpha = 1\nb = 22\n").expect("writes");

        let status = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::FormatChange);
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
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "def foo(").expect("writes");

        let status = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow");

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
    fn format_diff_returns_clean_for_already_canonical_file() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, true),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn format_diff_returns_config_error_for_missing_path() {
        let tmp = TempDir::new().expect("tempdir");
        let missing = tmp.path().join("does_not_exist");

        let status = format_with_io(
            format_args(vec![missing], false, true),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow");

        assert_eq!(status, ExitStatus::ConfigError);
    }

    #[test]
    fn format_diff_returns_format_change_for_pending_change() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "alpha = 1\nb = 22\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, true),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

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
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
        let contents = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(contents, "x = 1\n");
    }

    #[test]
    fn format_paths_rewrite_emits_json_when_format_is_non_text() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "alpha = 1\nb = 22\n").expect("writes");

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
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "def foo(").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs without anyhow");

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn format_writes_and_returns_clean_for_pending_change() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "alpha = 1\nb = 22\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
        let after = std::fs::read_to_string(&file).expect("reads");
        assert_ne!(after, "alpha = 1\nb = 22\n");
    }

    #[test]
    fn format_writes_return_config_error_when_target_is_readonly() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "alpha = 1\nb = 22\n").expect("writes");
        let mut perms = std::fs::metadata(&file).expect("metadata").permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&file, perms).expect("set_permissions");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            false,
            &windowed(),
            io::empty(),
            Vec::<u8>::new(),
            io::sink(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::ConfigError);
    }
}

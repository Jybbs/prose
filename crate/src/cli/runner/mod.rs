//! Pipeline orchestration: load source, run, emit diagnostics, classify outcome.

use std::{
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use anstream::{
    AutoStream,
    stream::{AsLockedWrite, RawStream},
};
use anyhow::Context;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use ruff_notebook::NotebookIndex;
use ruff_python_ast::PySourceType;
use ruff_source_file::SourceFile;

use super::{
    args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter},
    exit_status::ExitStatus,
    output::Presentation,
};
use crate::{
    cache::{Anchor, Cache, Rewrite},
    config::Config,
    diagnostics::Diagnostic,
    pipeline::Pipeline,
};

mod diff;
mod notebook;
mod process;
mod report;
mod resolve;

use diff::{Heading, write_rewrite_diff};
use process::{apply_rewrite, process_path, process_paths, process_stdin, read_stdin};
use report::{emit_to_stdout, emitter_summary, finish, render_summary, status_from_outcomes};
use resolve::{ConfigResolver, Resolved};

/// One file's contribution to the run.
#[derive(Debug)]
enum FileOutcome {
    Done {
        cached: bool,
        diagnostics: Vec<Diagnostic>,
        file: SourceFile,
        notebook_index: Option<Box<NotebookIndex>>,
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

/// Which pipeline passes a CLI mode reads from a file, and the
/// invocation shape that follows from it. Every per-run flag the runner
/// needs is derived from this one value, so no call site carries a
/// second copy of the same fact.
#[derive(Clone, Copy, Debug)]
enum Pass {
    /// `format` with a `json`, `sarif`, or `github` output format: the
    /// as-written diagnostics and the rewrite, committed to the file.
    Both,
    /// `check`: the as-written diagnostics, no rewrite. `validate` adds
    /// the opt-in guard that rejects a rule whose output fails to
    /// re-parse or to compile.
    Diagnose { validate: bool },
    /// `format --diff`: the rewrite alone, rendered as a patch and left
    /// uncommitted.
    Preview,
    /// Plain `format`: the rewrite alone, committed to the file.
    Rewrite,
}

impl Pass {
    /// Which buffer this pass's diagnostics resolve against, keying its
    /// entries apart from a pass that reads the other one.
    fn anchor(self) -> Anchor {
        match self {
            Self::Both | Self::Diagnose { .. } => Anchor::AsWritten,
            Self::Preview | Self::Rewrite => Anchor::Rewritten,
        }
    }

    /// True where the pass reads and writes the run's cache.
    fn caches(self) -> bool {
        // A `--validate` check re-confirms the rewrite parses rather than
        // trusting an entry an earlier unvalidated run wrote, so it alone
        // bypasses the cache.
        !matches!(self, Self::Diagnose { validate: true })
    }

    /// True where the pass never reached a structured emitter, leaving
    /// the surviving-lint count to the closing stderr disclosure.
    fn discloses_lint(self) -> bool {
        matches!(self, Self::Preview | Self::Rewrite)
    }

    /// Which closing summary this pass's outcomes resolve into.
    fn mode(self) -> Mode {
        match self {
            Self::Both | Self::Rewrite => Mode::Reformat,
            Self::Diagnose { .. } => Mode::Check,
            Self::Preview => Mode::Preview,
        }
    }

    /// True where the pass commits its rewrite to the file. A committing
    /// run reports a rewrite it landed as clean rather than as pending,
    /// which is the same distinction, so the exit status reads this too.
    fn write_back(self) -> bool {
        matches!(self, Self::Both | Self::Rewrite)
    }
}

/// Per-run setup shared across every path the walker yields.
struct RunSetup {
    cache: Option<Cache>,
    cwd: Arc<Resolved>,
    resolver: ConfigResolver,
    verbose: bool,
}

impl RunSetup {
    /// The cache a `pass` run reads and writes, `None` where the pass
    /// bypasses it.
    fn cache_for(&self, pass: Pass) -> Option<&Cache> {
        self.cache.as_ref().filter(|_| pass.caches())
    }

    /// Walks `paths` under `pass`, committing each rewrite to disk where
    /// the pass writes back, then enforces the cache's caps where the run
    /// inserted at least one entry. A run that hit on every file left the
    /// directory the size it already was, and so did a write-back run
    /// whose every miss produced a rewrite it commits rather than stores.
    /// Neither can newly cross a cap, leaving the sweep to stat every
    /// entry for nothing.
    fn walked(&self, paths: &[PathBuf], pass: Pass) -> Vec<FileOutcome> {
        let outcomes = process_paths(paths, |path, source_type| {
            let outcome = process_path(path, source_type, self, pass);
            if pass.write_back() {
                apply_rewrite(path, outcome)
            } else {
                outcome
            }
        });
        if let Some(cache) = self.cache_for(pass).filter(|c| c.inserted()) {
            cache.compact();
        }
        outcomes
    }
}

pub(crate) fn check_with_io<R: Read, O: RawStream + AsLockedWrite, E: Write>(
    args: CheckArgs,
    verbose: bool,
    present: &Presentation,
    stdin: R,
    stdout: AutoStream<O>,
    mut stderr: E,
) -> anyhow::Result<ExitStatus> {
    let pass = Pass::Diagnose {
        validate: args.validate,
    };
    let setup = match build_run(
        args.common.rules,
        args.common.no_cache,
        pass.anchor(),
        verbose,
    ) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    let outcomes = if args.common.stdin {
        let source_type = stdin_source_type(args.common.stdin_filename.as_deref());
        let outcome = match read_stdin(stdin) {
            Ok(text) => process_stdin(text, source_type, &setup.cwd.pipeline, pass),
            Err(outcome) => outcome,
        };
        vec![outcome]
    } else {
        setup.walked(&args.paths, pass)
    };
    let summary = emitter_summary(&outcomes);
    emit_to_stdout(
        &outcomes,
        args.common.output_format,
        present,
        stdout,
        &summary,
    )?;
    let status = finish(&outcomes, setup.cache.is_some(), setup.verbose, pass);
    render_summary(&mut stderr, present, &outcomes, &summary, pass);
    Ok(status)
}

/// Runs `format`, writing formatted source and undecorated diffs to the
/// raw stream beneath `stdout` and every other surface through `stdout`.
pub(crate) fn format_with_io<R: Read, O: RawStream + AsLockedWrite, E: Write>(
    args: FormatArgs,
    verbose: bool,
    present: &Presentation,
    stdin: R,
    stdout: AutoStream<O>,
    mut stderr: E,
) -> anyhow::Result<ExitStatus> {
    let pass = format_pass(args.diff, args.common.output_format);
    let setup = match build_run(
        args.common.rules,
        args.common.no_cache,
        pass.anchor(),
        verbose,
    ) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    if args.common.stdin {
        let source_type = stdin_source_type(args.common.stdin_filename.as_deref());
        return format_stdin(
            read_stdin(stdin).map(|text| (text, source_type)),
            pass,
            args.common.output_format,
            present,
            &setup.cwd.pipeline,
            stdout,
            &mut stderr,
        );
    }
    if args.diff {
        format_paths_diff(
            &args.paths,
            &setup,
            present,
            stdout.into_inner(),
            &mut stderr,
        )
    } else {
        format_paths_rewrite(
            &args.paths,
            pass,
            args.common.output_format,
            &setup,
            present,
            stdout,
            &mut stderr,
        )
    }
}

/// Builds the run-level setup. The cwd's own config governs stdin input
/// and the cache settings, while each path input re-resolves its own
/// effective config through the resolver. The `anchor` the run reads
/// keys every prefix the resolver builds.
fn build_run(
    rules: RuleFilter,
    no_cache: bool,
    anchor: Anchor,
    verbose: bool,
) -> Result<RunSetup, ExitStatus> {
    let resolver = ConfigResolver::new(rules.select, rules.ignore, anchor);
    let config = super::load_config_or_status(resolver.notices())?;
    let cache = open_cache(&config, no_cache);
    let cwd = resolver.seed(&config);
    Ok(RunSetup {
        cache,
        cwd,
        resolver,
        verbose,
    })
}

/// Resolves how the diff heads each file, painting the `🧵` line only
/// where stdout carries color through.
fn diff_heading(present: &Presentation) -> Heading {
    if present.decorate_diff() {
        Heading::Thread {
            color: present.color,
        }
    } else {
        Heading::Patch
    }
}

/// Resolves which pipeline passes a `format` invocation reads. A
/// `--diff` previews the rewrite without committing it, a plain text
/// rewrite commits it, and a `json`, `sarif`, or `github` output format
/// also renders the as-written diagnostics beside the commit.
fn format_pass(diff: bool, format: OutputFormat) -> Pass {
    if diff {
        Pass::Preview
    } else if format.is_text() {
        Pass::Rewrite
    } else {
        Pass::Both
    }
}

fn format_paths_diff<O: Write, E: Write>(
    paths: &[PathBuf],
    setup: &RunSetup,
    present: &Presentation,
    stdout: O,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let outcomes = setup.walked(paths, Pass::Preview);
    let heading = diff_heading(present);
    // Each diff renders into its own buffer on the pool, the shape
    // `Text::emit` uses, and rayon's indexed collect hands the buffers
    // back in walk order for the serial write.
    let blocks: Vec<Vec<u8>> = outcomes
        .par_iter()
        .filter_map(|outcome| match outcome {
            FileOutcome::Done {
                file,
                notebook_index,
                rewrite: Rewrite::Changed(kind),
                ..
            } => Some((file, notebook_index, kind)),
            _ => None,
        })
        .map(|(file, notebook_index, kind)| {
            let mut block = Vec::new();
            write_rewrite_diff(
                &mut block,
                file.name(),
                file.source_text(),
                kind,
                notebook_index.as_deref(),
                heading,
            )?;
            Ok(block)
        })
        .collect::<anyhow::Result<_>>()?;
    let mut writer = BufWriter::new(stdout);
    for block in &blocks {
        writer.write_all(block).context("writing stdout")?;
    }
    writer.flush().context("flushing stdout")?;
    let summary = emitter_summary(&outcomes);
    let status = finish(
        &outcomes,
        setup.cache.is_some(),
        setup.verbose,
        Pass::Preview,
    );
    render_summary(stderr, present, &outcomes, &summary, Pass::Preview);
    Ok(status)
}

fn format_paths_rewrite<O: RawStream + AsLockedWrite, E: Write>(
    paths: &[PathBuf],
    pass: Pass,
    format: OutputFormat,
    setup: &RunSetup,
    present: &Presentation,
    stdout: AutoStream<O>,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let outcomes = setup.walked(paths, pass);
    let summary = emitter_summary(&outcomes);
    if !format.is_text() {
        emit_to_stdout(&outcomes, format, present, stdout, &summary)?;
    }
    let status = finish(&outcomes, setup.cache.is_some(), setup.verbose, pass);
    render_summary(stderr, present, &outcomes, &summary, pass);
    Ok(status)
}

fn format_stdin<O: RawStream + AsLockedWrite, E: Write>(
    input: Result<(String, PySourceType), FileOutcome>,
    pass: Pass,
    format: OutputFormat,
    present: &Presentation,
    pipeline: &Pipeline,
    writer: AutoStream<O>,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let diff = matches!(pass, Pass::Preview);
    let (outcome, original) = match input {
        Ok((text, source_type)) => (
            process_stdin(text.clone(), source_type, pipeline, pass),
            text,
        ),
        Err(outcome) => (outcome, String::new()),
    };
    let outcomes = std::slice::from_ref(&outcome);
    let summary = emitter_summary(outcomes);
    if let FileOutcome::Done {
        notebook_index,
        rewrite,
        ..
    } = &outcome
    {
        if diff {
            if let Rewrite::Changed(kind) = rewrite {
                let heading = diff_heading(present);
                write_rewrite_diff(
                    &mut writer.into_inner(),
                    "<stdin>",
                    &original,
                    kind,
                    notebook_index.as_deref(),
                    heading,
                )?;
            }
        } else if format.is_text() {
            let to_write: &[u8] = match rewrite {
                Rewrite::Changed(kind) => kind.written().as_bytes(),
                // A non-Python notebook carries no rewrite, so echo stdin verbatim.
                Rewrite::PassedOver | Rewrite::Skipped | Rewrite::Unchanged => original.as_bytes(),
            };
            writer
                .into_inner()
                .write_all(to_write)
                .context("writing stdout")?;
        } else {
            emit_to_stdout(outcomes, format, present, writer, &summary)?;
        }
    }
    let status = status_from_outcomes(outcomes, pass.write_back());
    render_summary(stderr, present, outcomes, &summary, pass);
    Ok(status)
}

fn has_format_change(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity.is_format())
}

fn open_cache(config: &Config, no_cache: bool) -> Option<Cache> {
    if no_cache || !config.cache.enabled {
        return None;
    }
    Cache::open()
        .map(|c| {
            c.with_max_entries(config.cache.max_entries)
                .with_max_size_mib(config.cache.max_size_mib)
        })
        .inspect_err(|e| eprintln!("warning: cache disabled: {e}"))
        .ok()
}

/// Resolves the source type of stdin input from a `--stdin-filename`,
/// defaulting to Python when none is given.
fn stdin_source_type(filename: Option<&Path>) -> PySourceType {
    filename
        .and_then(PySourceType::try_from_path)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{assert_matches, io};

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        cli::args::RunArgs,
        rule::RuleId,
        testing::{FailingWriter, write_pyproject},
    };

    /// The unaligned two-assignment source the runner tests reuse.
    /// `ALPHA` is SCREAMING_CASE and `B` a single character, so the lint
    /// rules pass it silently while `align-equals` still reshapes it.
    const UNALIGNED: &str = "ALPHA = 1\nB = 22\n";

    struct ErrorReader;

    impl Read for ErrorReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated stdin failure"))
        }
    }

    fn check_args(paths: Vec<PathBuf>, stdin: bool) -> CheckArgs {
        CheckArgs {
            common: RunArgs {
                no_cache: true,
                stdin,
                ..Default::default()
            },
            paths,
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
            common: RunArgs {
                no_cache: true,
                stdin,
                ..Default::default()
            },
            diff,
            paths,
        }
    }

    fn run_check(args: CheckArgs) -> ExitStatus {
        run_check_io(args, io::empty()).0
    }

    fn run_check_io<R: Read>(args: CheckArgs, stdin: R) -> (ExitStatus, Vec<u8>) {
        let mut stdout = Vec::new();
        let status = check_with_io(
            args,
            false,
            &Presentation::windowed(),
            stdin,
            AutoStream::never(&mut stdout),
            io::sink(),
        )
        .expect("runs without anyhow");
        (status, stdout)
    }

    fn run_format(args: FormatArgs) -> ExitStatus {
        run_format_io(args, io::empty()).0
    }

    /// Runs `format` against a stdout whose writes fail with a broken
    /// pipe, returning the `io::ErrorKind` the propagated error carries.
    fn run_format_broken_pipe<R: Read>(args: FormatArgs, stdin: R) -> Option<io::ErrorKind> {
        let mut failing = FailingWriter(io::ErrorKind::BrokenPipe);
        let writer: &mut dyn Write = &mut failing;
        format_with_io(
            args,
            false,
            &Presentation::windowed(),
            stdin,
            AutoStream::never(writer),
            io::sink(),
        )
        .expect_err("writer failure propagates")
        .downcast_ref::<io::Error>()
        .map(io::Error::kind)
    }

    /// Runs `format` against `stdin` through a stripping stdout.
    fn run_format_io<R: Read>(args: FormatArgs, stdin: R) -> (ExitStatus, Vec<u8>) {
        let mut stdout = Vec::new();
        let status = format_with_io(
            args,
            false,
            &Presentation::windowed(),
            stdin,
            AutoStream::never(&mut stdout),
            io::sink(),
        )
        .expect("runs without anyhow");
        (status, stdout)
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
        args.common.output_format = OutputFormat::Json;

        let (status, stdout) = run_check_io(args, io::empty());

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
        let (_tmp, file) = fixture(UNALIGNED);

        let mut args = check_args(vec![file], false);
        args.common.rules.ignore = vec![RuleId::from("align-equals")];

        assert_eq!(run_check(args), ExitStatus::Clean);
    }

    #[test]
    fn check_pending_format_returns_format_change() {
        let (tmp, _file) = fixture(UNALIGNED);

        let status = run_check(check_args(vec![tmp.path().to_path_buf()], false));

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn check_resolves_config_from_the_files_own_ancestors() {
        let (tmp, file) = fixture(UNALIGNED);
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        assert_eq!(run_check(check_args(vec![file], false)), ExitStatus::Clean);
    }

    #[test]
    fn check_select_overrides_the_files_own_config() {
        let (tmp, file) = fixture(UNALIGNED);
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        let mut args = check_args(vec![file], false);
        args.common.rules.select = vec![RuleId::from("align-equals")];

        assert_eq!(run_check(args), ExitStatus::FormatChange);
    }

    #[test]
    fn check_stdin_returns_clean_for_canonical_source() {
        let (status, _) = run_check_io(check_args(Vec::new(), true), b"x = 1\n".as_slice());

        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn check_stdin_with_read_failure_returns_config_error() {
        let (status, _) = run_check_io(check_args(Vec::new(), true), ErrorReader);

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
        let (status, _) = run_check_io(check_args(Vec::new(), true), b"def foo(".as_slice());

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn diff_heading_falls_back_to_the_patch_header_off_a_tty() {
        assert_matches!(diff_heading(&Presentation::windowed()), Heading::Patch);
    }

    #[rstest]
    #[case::color_on(true, true)]
    #[case::color_off(false, false)]
    fn diff_heading_paints_the_thread_line_only_under_color(
        #[case] color: bool,
        #[case] painted: bool,
    ) {
        let present = Presentation {
            color,
            quiet: false,
            stdout_tty: true,
        };

        assert_matches!(
            diff_heading(&present),
            Heading::Thread { color } if color == painted
        );
    }

    #[test]
    fn format_diff_resolves_config_from_the_files_own_ancestors() {
        let (tmp, file) = fixture(UNALIGNED);
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
        let (tmp, _file) = fixture(UNALIGNED);

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, true));

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn format_pass_needs_both_only_for_a_diagnostic_format() {
        assert_matches!(format_pass(false, OutputFormat::Json), Pass::Both);
        assert_matches!(format_pass(false, OutputFormat::Text), Pass::Rewrite);
        assert_matches!(format_pass(true, OutputFormat::Json), Pass::Preview);
        assert_matches!(format_pass(true, OutputFormat::Text), Pass::Preview);
    }

    #[test]
    fn format_paths_diff_propagates_a_broken_pipe() {
        let (tmp, _file) = fixture(UNALIGNED);

        let kind = run_format_broken_pipe(
            format_args(vec![tmp.path().to_path_buf()], false, true),
            io::empty(),
        );

        assert_eq!(kind, Some(io::ErrorKind::BrokenPipe));
    }

    #[test]
    fn format_paths_leaves_a_canonical_file_unchanged() {
        let (tmp, file) = fixture("x = 1\n");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::Clean);
        let contents = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(contents, "x = 1\n");
    }

    #[test]
    fn format_paths_rewrite_emits_json_when_format_is_non_text() {
        let (tmp, _file) = fixture(UNALIGNED);

        let mut args = format_args(vec![tmp.path().to_path_buf()], false, false);
        args.common.output_format = OutputFormat::Json;

        let (status, stdout) = run_format_io(args, io::empty());

        assert_eq!(status, ExitStatus::Clean);
        assert!(!stdout.is_empty());
    }

    #[test]
    fn format_rewrite_resolves_config_from_the_files_own_ancestors() {
        let (tmp, file) = fixture(UNALIGNED);
        write_pyproject(tmp.path(), "[tool.prose.rules]\nalign-equals = false\n");

        let status = run_format(format_args(vec![file.clone()], false, false));

        assert_eq!(status, ExitStatus::Clean);
        let after = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(after, UNALIGNED);
    }

    #[test]
    fn format_stdin_diff_propagates_a_broken_pipe() {
        let kind =
            run_format_broken_pipe(format_args(Vec::new(), true, true), UNALIGNED.as_bytes());

        assert_eq!(kind, Some(io::ErrorKind::BrokenPipe));
    }

    #[test]
    fn format_stdin_diff_writes_unified_hunks() {
        let (status, stdout) =
            run_format_io(format_args(Vec::new(), true, true), UNALIGNED.as_bytes());

        assert_eq!(status, ExitStatus::FormatChange);
        let out = String::from_utf8(stdout).expect("utf-8");
        assert!(out.contains("@@"));
    }

    #[test]
    fn format_stdin_emits_json_when_format_is_non_text() {
        let mut args = format_args(Vec::new(), true, false);
        args.common.output_format = OutputFormat::Json;

        let (status, stdout) = run_format_io(args, UNALIGNED.as_bytes());

        assert_eq!(status, ExitStatus::Clean);
        assert!(!stdout.is_empty());
    }

    #[test]
    fn format_stdin_prints_canonical_source_verbatim() {
        let (status, stdout) =
            run_format_io(format_args(Vec::new(), true, false), b"x = 1\n".as_slice());

        assert_eq!(status, ExitStatus::Clean);
        assert_eq!(stdout, b"x = 1\n");
    }

    #[test]
    fn format_stdin_propagates_a_broken_pipe_from_the_raw_stream() {
        let kind =
            run_format_broken_pipe(format_args(Vec::new(), true, false), UNALIGNED.as_bytes());

        assert_eq!(kind, Some(io::ErrorKind::BrokenPipe));
    }

    #[test]
    fn format_stdin_with_read_failure_returns_config_error() {
        let (status, _) = run_format_io(format_args(Vec::new(), true, false), ErrorReader);

        assert_eq!(status, ExitStatus::ConfigError);
    }

    #[rstest]
    #[case::bell("x = \"a\u{7}b\"\n")]
    #[case::comment("# note \u{1b}here\nx = 1\n")]
    #[case::embedded("x = \"AB\u{1b}CD\"\n")]
    #[case::lone("x = \"\u{1b}\"\n")]
    fn format_stdin_writes_escape_bytes_to_the_raw_stream(#[case] source: &str) {
        let (status, stdout) =
            run_format_io(format_args(Vec::new(), true, false), source.as_bytes());

        assert_eq!(status, ExitStatus::Clean);
        assert_eq!(stdout, source.as_bytes());
    }

    #[test]
    fn format_stdin_writes_the_rewrite_to_the_raw_stream() {
        let source = "AB = \"X\u{1b}Y\"\nc = 2\n";

        let (status, stdout) =
            run_format_io(format_args(Vec::new(), true, false), source.as_bytes());

        assert_eq!(status, ExitStatus::Clean);
        assert_eq!(stdout, "AB = \"X\u{1b}Y\"\nc  = 2\n".as_bytes());
    }

    #[test]
    fn format_unparseable_returns_parse_error() {
        let (tmp, _file) = fixture("def foo(");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn format_writes_and_returns_clean_for_pending_change() {
        let (tmp, file) = fixture(UNALIGNED);

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::Clean);
        let after = std::fs::read_to_string(&file).expect("reads");
        assert_ne!(after, UNALIGNED);
    }

    #[test]
    fn format_writes_return_config_error_when_target_is_readonly() {
        let (tmp, file) = fixture(UNALIGNED);
        let mut perms = std::fs::metadata(&file).expect("metadata").permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&file, perms).expect("set_permissions");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::ConfigError);
    }
    #[rstest]
    #[case(Pass::Both, Anchor::AsWritten)]
    #[case(Pass::Diagnose { validate: false }, Anchor::AsWritten)]
    #[case(Pass::Diagnose { validate: true }, Anchor::AsWritten)]
    #[case(Pass::Preview, Anchor::Rewritten)]
    #[case(Pass::Rewrite, Anchor::Rewritten)]
    fn pass_anchors_only_a_rewriting_pass_against_the_output(
        #[case] pass: Pass,
        #[case] anchor: Anchor,
    ) {
        assert_eq!(pass.anchor(), anchor);
    }

    #[rstest]
    #[case(Pass::Both, true)]
    #[case(Pass::Diagnose { validate: false }, true)]
    #[case(Pass::Diagnose { validate: true }, false)]
    #[case(Pass::Preview, true)]
    #[case(Pass::Rewrite, true)]
    fn pass_caches_every_shape_but_a_validated_check(#[case] pass: Pass, #[case] caches: bool) {
        assert_eq!(pass.caches(), caches);
    }

    #[rstest]
    #[case(Pass::Both, false)]
    #[case(Pass::Diagnose { validate: false }, false)]
    #[case(Pass::Preview, true)]
    #[case(Pass::Rewrite, true)]
    fn pass_discloses_lint_only_where_no_emitter_printed_it(
        #[case] pass: Pass,
        #[case] discloses: bool,
    ) {
        assert_eq!(pass.discloses_lint(), discloses);
    }

    #[rstest]
    #[case(Pass::Both, Mode::Reformat)]
    #[case(Pass::Diagnose { validate: false }, Mode::Check)]
    #[case(Pass::Preview, Mode::Preview)]
    #[case(Pass::Rewrite, Mode::Reformat)]
    fn pass_resolves_its_closing_summary_mode(#[case] pass: Pass, #[case] expected: Mode) {
        assert_eq!(
            std::mem::discriminant(&pass.mode()),
            std::mem::discriminant(&expected),
        );
    }

    #[rstest]
    #[case(Pass::Both, true)]
    #[case(Pass::Diagnose { validate: false }, false)]
    #[case(Pass::Preview, false)]
    #[case(Pass::Rewrite, true)]
    fn pass_writes_back_only_where_it_commits_the_rewrite(
        #[case] pass: Pass,
        #[case] write_back: bool,
    ) {
        assert_eq!(pass.write_back(), write_back);
    }
}

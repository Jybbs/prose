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
use ruff_notebook::NotebookIndex;
use ruff_python_ast::PySourceType;
use ruff_source_file::SourceFile;

use super::{
    args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter, RunArgs},
    exit_status::ExitStatus,
    output::Presentation,
};
use crate::{
    cache::{Anchor, Cache, Rewrite},
    cli::emit::{EmitterSummary, Run, Text},
    config::Config,
    diagnostics::Diagnostic,
    unstable::UnstableRewrite,
};

mod diff;
mod notebook;
mod pass;
mod process;
mod report;
mod resolve;
mod stdin;
mod stdout;
mod unstable;
mod walk;
mod write;

use diff::{Heading, write_rewrite_diff};
use pass::{Mode, Pass};
use process::process_path;
use report::{emitter_summary, finish, render_summary, unstable_status};
use resolve::{ConfigResolver, Resolved};
use stdin::{format_stdin, process_stdin, read_stdin, stdin_resolved};
use stdout::{emit_to_stdout, render_text_block};
use walk::{process_paths, stream_paths};
use write::apply_rewrite;

/// The diagnostic name a stdin run carries, rendered back as the `-`
/// positional wherever a report names an invocation.
const STDIN_NAME: &str = "<stdin>";

/// One file's contribution to the run.
#[derive(Debug)]
enum FileOutcome {
    Done {
        cached: bool,
        diagnostics: Vec<Diagnostic>,
        file: SourceFile,
        notebook_index: Option<Box<NotebookIndex>>,
        rewrite: Rewrite,
        unstable: Option<Box<UnstableRewrite>>,
    },
    Failed(ExitStatus),
}

impl FileOutcome {
    /// Renders this outcome's unified diff under `heading`, empty where
    /// the rewrite left the file as written.
    fn diff_block(&self, heading: Heading) -> anyhow::Result<Vec<u8>> {
        let Self::Done {
            file,
            notebook_index,
            rewrite: Rewrite::Changed(kind),
            ..
        } = self
        else {
            return Ok(Vec::new());
        };
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
    }

    /// The emitter run this outcome renders through, `None` for a file
    /// that failed.
    fn run(&self) -> Option<Run<'_>> {
        match self {
            Self::Done {
                diagnostics,
                file,
                notebook_index,
                ..
            } => Some(Run::new(file, diagnostics, notebook_index.as_deref())),
            Self::Failed(_) => None,
        }
    }

    /// The source file and settle report this outcome carries, `None`
    /// for a settled rewrite and for a file that failed.
    fn unstable(&self) -> Option<(&SourceFile, &UnstableRewrite)> {
        match self {
            Self::Done { file, unstable, .. } => unstable.as_deref().map(|rewrite| (file, rewrite)),
            Self::Failed(_) => None,
        }
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
    /// Builds the run-level setup. The cwd's own config governs stdin
    /// input and the cache settings, while each path input re-resolves
    /// its own effective config through the resolver. The `anchor` the
    /// run reads keys every prefix the resolver builds.
    fn new(
        rules: &RuleFilter,
        no_cache: bool,
        anchor: Anchor,
        verbose: bool,
    ) -> Result<Self, ExitStatus> {
        let resolver = ConfigResolver::new(rules.select.clone(), rules.ignore.clone(), anchor);
        let config = super::load_config_or_status(resolver.notices())?;
        let cache = open_cache(&config, no_cache);
        let cwd = resolver.seed(&config);
        Ok(Self {
            cache,
            cwd,
            resolver,
            verbose,
        })
    }

    /// The cache a `pass` run reads and writes, `None` where the pass
    /// bypasses it.
    fn cache_for(&self, pass: Pass) -> Option<&Cache> {
        self.cache.as_ref().filter(|_| pass.caches())
    }

    /// Enforces the cache's caps where the run inserted at least one
    /// entry.
    fn compact_after(&self, pass: Pass) {
        if let Some(cache) = self.cache_for(pass).filter(|c| c.inserted()) {
            cache.compact();
        }
    }

    /// Walks `paths` under `pass`, rendering each file's block through
    /// `render` in the worker that produced its outcome and handing it
    /// to `write` in walker order as it lands. No rewrite reaches disk
    /// here.
    fn streamed<R, W>(
        &self,
        paths: &[PathBuf],
        pass: Pass,
        render: R,
        write: W,
    ) -> anyhow::Result<Vec<FileOutcome>>
    where
        R: Fn(&FileOutcome) -> anyhow::Result<Vec<u8>> + Send + Sync,
        W: FnMut(&[u8]) -> anyhow::Result<()>,
    {
        let outcomes = stream_paths(
            paths,
            pass,
            |path, source_type| {
                let outcome = process_path(path, source_type, self, pass);
                let block = render(&outcome)?;
                Ok((outcome, block))
            },
            write,
        )?;
        self.compact_after(pass);
        Ok(outcomes)
    }

    /// Walks `paths` under `pass`, committing each rewrite to disk where
    /// the pass writes back, then enforcing the cache's caps.
    fn walked(&self, paths: &[PathBuf], pass: Pass) -> Vec<FileOutcome> {
        let outcomes = process_paths(paths, pass, |path, source_type| {
            let outcome = process_path(path, source_type, self, pass);
            if pass.write_back() {
                apply_rewrite(path, outcome)
            } else {
                outcome
            }
        });
        self.compact_after(pass);
        outcomes
    }
}

pub(crate) fn check_with_io<R: Read, O: RawStream + AsLockedWrite, E: Write>(
    args: CheckArgs,
    verbose: bool,
    present: &Presentation,
    stdin: R,
    mut stdout: AutoStream<O>,
    mut stderr: E,
) -> anyhow::Result<ExitStatus> {
    let pass = Pass::Diagnose {
        validate: args.validate,
    };
    let setup = match RunSetup::new(
        &args.common.rules,
        args.common.no_cache,
        pass.anchor(),
        verbose,
    ) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    let format = args.common.output_format;
    let (outcomes, emits) = if args.common.stdin {
        let filename = args.common.stdin_filename.as_deref();
        let outcome = match read_stdin(stdin) {
            Ok(text) => {
                let resolved = stdin_resolved(&setup, filename, &text);
                process_stdin(text, filename, &resolved, pass)
            }
            Err(outcome) => outcome,
        };
        (vec![outcome], true)
    } else if format.is_text() {
        // Text carries no envelope around its per-file blocks, so each
        // one is written as it lands, whereas every other format closes
        // over the run.
        let text = Text::new(present.color);
        let outcomes = setup.streamed(
            &args.paths,
            pass,
            |outcome| render_text_block(&text, outcome),
            |block| stdout.write_all(block).context("writing stdout"),
        )?;
        stdout.flush().context("flushing stdout")?;
        (outcomes, false)
    } else {
        (setup.walked(&args.paths, pass), true)
    };
    let summary = emitter_summary(&outcomes);
    if emits {
        emit_to_stdout(&outcomes, format, present, stdout, &summary)?;
    }
    let status = close_run(&outcomes, &summary, &setup, present, pass, &mut stderr)
        .max(unstable_status(&outcomes));
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
    let pass = Pass::for_format(args.diff, args.common.output_format);
    let setup = match RunSetup::new(
        &args.common.rules,
        args.common.no_cache,
        pass.anchor(),
        verbose,
    ) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    if args.common.stdin {
        return format_stdin(
            read_stdin(stdin),
            &args.common,
            pass,
            present,
            &setup,
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

/// Closes a run, rendering its summary onto `stderr` and resolving the
/// status it exits with.
fn close_run<E: Write>(
    outcomes: &[FileOutcome],
    summary: &EmitterSummary,
    setup: &RunSetup,
    present: &Presentation,
    pass: Pass,
    stderr: &mut E,
) -> ExitStatus {
    let status = finish(outcomes, setup.cache.is_some(), setup.verbose, pass, stderr);
    render_summary(stderr, present, outcomes, summary, pass);
    status
}

fn format_paths_diff<O: Write, E: Write>(
    paths: &[PathBuf],
    setup: &RunSetup,
    present: &Presentation,
    stdout: O,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let heading = Heading::from(present);
    let mut writer = BufWriter::new(stdout);
    // Each diff renders in the worker that produced its outcome and
    // reaches the buffer in walk order as it lands.
    let outcomes = setup.streamed(
        paths,
        Pass::Preview,
        |outcome| outcome.diff_block(heading),
        |block| writer.write_all(block).context("writing stdout"),
    )?;
    writer.flush().context("flushing stdout")?;
    let summary = emitter_summary(&outcomes);
    Ok(close_run(
        &outcomes,
        &summary,
        setup,
        present,
        Pass::Preview,
        stderr,
    ))
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
    Ok(close_run(&outcomes, &summary, setup, present, pass, stderr))
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

#[cfg(test)]
mod tests {
    use std::io;

    use rstest::rstest;
    use ruff_diagnostics::{Edit, Fix};
    use ruff_text_size::TextRange;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        diagnostics::Severity,
        rules::RuleId,
        source::Source,
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

    pub(super) fn diagnostic(
        severity: Severity,
        range: TextRange,
        slug: &'static str,
    ) -> Diagnostic {
        Diagnostic {
            fix: severity
                .is_format()
                .then(|| Fix::safe_edit(Edit::range_replacement("y".into(), range))),
            message: "test".into(),
            range,
            rule: RuleId::from(slug),
            severity,
        }
    }

    pub(super) fn outcome_with(source: Source, diagnostics: Vec<Diagnostic>) -> FileOutcome {
        FileOutcome::Done {
            cached: false,
            diagnostics,
            file: source.source_file().clone(),
            notebook_index: None,
            rewrite: Rewrite::Skipped,
            unstable: None,
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
        fs_err::write(&file, source).expect("writes");
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

    /// Parses the summary record a JSON run closes its stdout with.
    fn json_summary(stdout: Vec<u8>) -> serde_json::Value {
        let out = String::from_utf8(stdout).expect("utf-8");
        serde_json::from_str(out.lines().last().expect("a summary line")).expect("parses")
    }

    fn run_check(args: CheckArgs) -> ExitStatus {
        run_check_io(args, io::empty()).0
    }

    fn run_check_io<R: Read>(args: CheckArgs, stdin: R) -> (ExitStatus, Vec<u8>) {
        let mut stdout = Vec::new();
        let status = run_io(args, stdin, &mut stdout, check_with_io);
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
        let status = run_io(args, stdin, &mut stdout, format_with_io);
        (status, stdout)
    }

    /// Runs `run`, an entry point, over `args` and `stdin` with `stdout`
    /// as its stripping stdout, returning its status.
    fn run_io<'b, A, R: Read>(
        args: A,
        stdin: R,
        stdout: &'b mut Vec<u8>,
        run: impl FnOnce(
            A,
            bool,
            &Presentation,
            R,
            AutoStream<&'b mut Vec<u8>>,
            io::Sink,
        ) -> anyhow::Result<ExitStatus>,
    ) -> ExitStatus {
        run(
            args,
            false,
            &Presentation::windowed(),
            stdin,
            AutoStream::never(stdout),
            io::sink(),
        )
        .expect("runs without anyhow")
    }

    #[test]
    fn check_broken_ancestor_config_fails_the_file_and_continues() {
        let tmp = TempDir::new().expect("tempdir");
        let broken = tmp.path().join("broken");
        let plain = tmp.path().join("plain");
        fs_err::create_dir_all(&plain).expect("dirs create");
        write_pyproject(&broken, "[this is not valid TOML");
        fs_err::write(broken.join("a.py"), "x = 1\n").expect("writes");
        fs_err::write(plain.join("b.py"), "x = 1\n").expect("writes");

        let mut args = check_args(vec![broken.join("a.py"), plain.join("b.py")], false);
        args.common.output_format = OutputFormat::Json;

        let (status, stdout) = run_check_io(args, io::empty());

        assert_eq!(status, ExitStatus::ConfigError);
        assert_eq!(json_summary(stdout)["files_visited"], 1);
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
    fn check_rejects_a_named_script_and_checks_the_other_named_files() {
        let (tmp, file) = fixture("x = 1\n");
        let script = tmp.path().join("script");
        fs_err::write(&script, "x = 1\n").expect("writes");

        let mut args = check_args(vec![script, file], false);
        args.common.output_format = OutputFormat::Json;

        let (status, stdout) = run_check_io(args, io::empty());

        assert_eq!(status, ExitStatus::ConfigError);
        assert_eq!(json_summary(stdout)["files_visited"], 1);
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
        let contents = fs_err::read_to_string(&file).expect("reads");
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
        let after = fs_err::read_to_string(&file).expect("reads");
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
        let after = fs_err::read_to_string(&file).expect("reads");
        assert_ne!(after, UNALIGNED);
    }

    #[test]
    fn format_writes_return_config_error_when_target_is_readonly() {
        let (tmp, file) = fixture(UNALIGNED);
        let mut perms = fs_err::metadata(&file).expect("metadata").permissions();
        perms.set_readonly(true);
        fs_err::set_permissions(&file, perms).expect("set_permissions");

        let status = run_format(format_args(vec![tmp.path().to_path_buf()], false, false));

        assert_eq!(status, ExitStatus::ConfigError);
    }
}

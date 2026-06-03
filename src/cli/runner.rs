//! Pipeline orchestration: load source, run, emit diagnostics, classify outcome.

use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use rayon::iter::{ParallelBridge, ParallelIterator};
use ruff_source_file::{SourceFile, SourceFileBuilder};

use super::{
    args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter},
    exit_status::ExitStatus,
    output::{self, Presentation, Summary},
};
use crate::{
    cache::{Cache, CacheEntry, CacheKey, Rewrite},
    config::Config,
    diagnostics::{Diagnostic, Emitter, EmitterSummary, Github, Json, Run, Sarif, Severity, Text},
    pipeline::Pipeline,
    source::Source,
    walker,
};

/// One file's contribution to the run.
#[derive(Debug)]
enum FileOutcome {
    Done {
        cached: bool,
        diagnostics: Vec<Diagnostic>,
        file: SourceFile,
        formatted_text: Option<String>,
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

fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    let FileOutcome::Done {
        formatted_text: Some(text),
        ..
    } = &outcome
    else {
        return outcome;
    };
    if let Err(e) = fs_err::write(path, text) {
        return config_error(e);
    }
    outcome
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

/// Records what a cached entry knows about the rewrite. A mode that
/// skipped `run` stores `Skipped`, whereas one that ran it records
/// whether the text changed.
fn cache_rewrite(needs_rewrite: bool, formatted_text: Option<&str>) -> Rewrite {
    if !needs_rewrite {
        return Rewrite::Skipped;
    }
    match formatted_text {
        Some(text) => Rewrite::Changed(text.to_owned()),
        None => Rewrite::Unchanged,
    }
}

fn config_error(e: impl std::fmt::Display) -> FileOutcome {
    eprintln!("error: {e}");
    FileOutcome::Failed(ExitStatus::ConfigError)
}

fn emit_outcomes<W: Write>(
    outcomes: &[FileOutcome],
    format: OutputFormat,
    writer: &mut W,
    summary: &EmitterSummary,
) -> anyhow::Result<()> {
    let view: Vec<Run<'_>> = outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Done {
                file, diagnostics, ..
            } => Some((file, diagnostics.as_slice())),
            FileOutcome::Failed(_) => None,
        })
        .collect();
    match format {
        OutputFormat::Github => Github.emit(writer, &view, summary),
        OutputFormat::Json => Json.emit(writer, &view, summary),
        OutputFormat::Sarif => Sarif.emit(writer, &view, summary),
        OutputFormat::Text => Text::new().emit(writer, &view, summary),
    }?;
    writer.flush().context("flushing stdout")?;
    Ok(())
}

fn emitter_summary(outcomes: &[FileOutcome]) -> EmitterSummary {
    outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Done {
                diagnostics,
                formatted_text,
                ..
            } => Some((diagnostics, formatted_text)),
            FileOutcome::Failed(_) => None,
        })
        .fold(
            EmitterSummary::default(),
            |mut summary, (diagnostics, formatted_text)| {
                summary.files_visited += 1;
                summary.files_changed +=
                    usize::from(file_changed(diagnostics, formatted_text.as_deref()));
                summary.files_with_diagnostics += usize::from(!diagnostics.is_empty());
                summary.diagnostics_total += diagnostics.len();
                for diag in diagnostics {
                    *summary.rules_fired.entry(diag.rule).or_default() += 1;
                }
                summary
            },
        )
}

/// A file counts as changed when `run` produced a rewrite, or, on the
/// `check` path that skips the rewrite, when `diagnose` emitted a format
/// diagnostic.
fn file_changed(diagnostics: &[Diagnostic], formatted_text: Option<&str>) -> bool {
    formatted_text.is_some() || has_format_change(diagnostics)
}

fn finish(
    outcomes: &[FileOutcome],
    setup: &RunSetup,
    verbose: bool,
    demote_format_change: bool,
) -> ExitStatus {
    if verbose {
        report_verbose(outcomes, setup.cache.is_some(), &mut io::stderr());
    }
    status_from_outcomes(outcomes, demote_format_change)
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
            formatted_text: Some(formatted),
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
    if let FileOutcome::Done {
        file,
        formatted_text,
        ..
    } = &outcome
    {
        if diff {
            if let Some(formatted) = formatted_text {
                write_diff(
                    writer,
                    "<stdin>",
                    file.source_text(),
                    formatted,
                    present.decorate_diff(),
                )?;
            }
        } else if format.is_text() {
            let to_write = formatted_text.as_deref().unwrap_or(file.source_text());
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

fn process_path(path: &Path, setup: &RunSetup, pass: Pass) -> FileOutcome {
    let bytes = match fs_err::read(path) {
        Ok(b) => b,
        Err(e) => return config_error(e),
    };
    // Plain `format` would persist only `run`'s post-edit diagnostics, and
    // a `--validate` check must re-confirm the rewrite parses rather than
    // trust an entry an earlier unvalidated run wrote, so both bypass the
    // cache. Every entry that remains carries `diagnose`'s as-written
    // diagnostics, so a `check` hit never replays a `run`'s.
    let needs_rewrite = matches!(pass, Pass::Both);
    let keyed = setup
        .cache
        .as_ref()
        .filter(|_| !matches!(pass, Pass::Rewrite | Pass::Diagnose { validate: true }))
        .map(|c| (c, CacheKey::compute(&bytes, &setup.config_toml)));
    if let Some(outcome) = keyed
        .as_ref()
        .and_then(|(c, k)| c.lookup(k))
        .and_then(|entry| rehydrate(path, &bytes, entry, needs_rewrite))
    {
        return outcome;
    }
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {} is not valid UTF-8: {e}", path.display());
            return FileOutcome::Failed(ExitStatus::ConfigError);
        }
    };
    let source = match Source::build(text, path.display().to_string()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: parse error in `{}`: {e}", path.display());
            return FileOutcome::Failed(ExitStatus::ParseError);
        }
    };
    let outcome = run_pipeline(source, &setup.pipeline, pass);
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            formatted_text,
            ..
        },
    ) = (&keyed, &outcome)
    {
        c.insert(
            k,
            &CacheEntry {
                diagnostics: diagnostics.clone(),
                rewrite: cache_rewrite(needs_rewrite, formatted_text.as_deref()),
            },
        );
    }
    outcome
}

fn process_paths<F>(paths: &[PathBuf], handle: F) -> Vec<FileOutcome>
where
    F: Fn(&Path) -> FileOutcome + Send + Sync,
{
    walker::walk(paths)
        .par_bridge()
        .map(|entry| entry.map_or_else(walk_error, |path| handle(&path)))
        .collect()
}

fn process_stdin<R: Read>(stdin: R, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let Ok(text) =
        io::read_to_string(stdin).inspect_err(|e| eprintln!("error: reading stdin: {e}"))
    else {
        return FileOutcome::Failed(ExitStatus::ConfigError);
    };
    text.parse::<Source>()
        .inspect_err(|e| eprintln!("error: parse error in stdin: {e}"))
        .map_or_else(
            |_| FileOutcome::Failed(ExitStatus::ParseError),
            |source| run_pipeline(source, pipeline, pass),
        )
}

fn rehydrate(
    path: &Path,
    original_bytes: &[u8],
    entry: CacheEntry,
    needs_rewrite: bool,
) -> Option<FileOutcome> {
    let formatted_text = if needs_rewrite {
        match entry.rewrite {
            Rewrite::Changed(text) => Some(text),
            // A `check` entry skipped the rewrite this mode needs.
            Rewrite::Skipped => return None,
            Rewrite::Unchanged => None,
        }
    } else {
        None
    };
    let original_text = std::str::from_utf8(original_bytes).ok()?.to_owned();
    let file = SourceFileBuilder::new(path.display().to_string(), original_text).finish();
    Some(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        formatted_text,
    })
}

fn render_summary<E: Write>(stderr: &mut E, present: &Presentation, summary: Option<Summary>) {
    if let Some(summary) = summary {
        let _ = output::report(stderr, present, &summary);
    }
}

fn report_verbose<W: Write>(outcomes: &[FileOutcome], cache_enabled: bool, writer: &mut W) {
    if !cache_enabled {
        let _ = writeln!(writer, "cache: bypassed");
        return;
    }
    let (hits, misses) = outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Done { cached, .. } => Some(*cached),
            FileOutcome::Failed(_) => None,
        })
        .fold(
            (0_usize, 0_usize),
            |(h, m), c| if c { (h + 1, m) } else { (h, m + 1) },
        );
    let _ = writeln!(
        writer,
        "cache: {hits} hits, {misses} misses, {total} files",
        total = hits + misses,
    );
}

/// Computes only the passes `pass` reads. `check` collects the
/// as-written diagnostics, and with `--validate` set it also guards the
/// would-be rewrite against an unparseable rule output without rebuilding
/// diagnostics. A `format` run rewrites through `run`, pairing the
/// rewrite with `diagnose`'s as-written diagnostics when an output format
/// will render them, or `run`'s own otherwise.
fn run_pipeline(source: Source, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let file = source.source_file().clone();
    if let Pass::Diagnose { validate } = pass {
        let diagnostics = pipeline.diagnose(&source);
        if validate
            && has_format_change(&diagnostics)
            && let Err(e) = pipeline.validate(source)
        {
            return config_error(e);
        }
        return FileOutcome::Done {
            cached: false,
            diagnostics,
            file,
            formatted_text: None,
        };
    }
    let diagnosed = matches!(pass, Pass::Both).then(|| pipeline.diagnose(&source));
    match pipeline.run(source) {
        Ok((formatted, run_diagnostics)) => {
            let formatted_text = formatted
                .changed_from(file.source_text())
                .map(str::to_owned);
            FileOutcome::Done {
                cached: false,
                diagnostics: diagnosed.unwrap_or(run_diagnostics),
                file,
                formatted_text,
            }
        }
        Err(e) => config_error(e),
    }
}

fn status_from_outcomes(outcomes: &[FileOutcome], demote_format_change: bool) -> ExitStatus {
    outcomes
        .iter()
        .map(|outcome| match outcome {
            FileOutcome::Done { diagnostics, .. } => diagnostics
                .iter()
                .map(|d| ExitStatus::from(d.severity))
                .filter(|s| !demote_format_change || *s != ExitStatus::FormatChange)
                .max()
                .unwrap_or_default(),
            FileOutcome::Failed(s) => *s,
        })
        .max()
        .unwrap_or_default()
}

/// Resolves an outcome set into its closing [`Summary`], or `None`
/// when a clean run is shadowed by a per-file failure already logged
/// to stderr.
fn summarize(outcomes: &[FileOutcome], summary: &EmitterSummary, mode: Mode) -> Option<Summary> {
    let failed = outcomes.iter().any(|o| matches!(o, FileOutcome::Failed(_)));
    let resolved = match mode {
        Mode::Check => match summary.diagnostics_total {
            0 => Summary::Clean,
            total => Summary::Diagnostics {
                files: summary.files_with_diagnostics,
                total,
            },
        },
        Mode::Preview => match summary.files_changed {
            0 => Summary::Clean,
            files => Summary::WouldReformat { files },
        },
        Mode::Reformat => match summary.files_changed {
            0 => Summary::Clean,
            files => Summary::Reformatted { files },
        },
    };
    match resolved {
        Summary::Clean if failed => None,
        resolved => Some(resolved),
    }
}

fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    eprintln!("error: cannot walk: {err}");
    FileOutcome::Failed(ExitStatus::ConfigError)
}

/// Writes a unified diff between `before` and `after`. When
/// `decorate`, a Ube `🧵 <name>` heading stands in for the plain
/// `---`/`+++` header, which is reserved for off-TTY runs so the diff
/// stays a valid patch.
fn write_diff<W: Write>(
    writer: &mut W,
    name: &str,
    before: &str,
    after: &str,
    decorate: bool,
) -> anyhow::Result<()> {
    let diff = similar::TextDiff::from_lines(before, after);
    let mut unified = diff.unified_diff();
    if decorate {
        writeln!(writer, "{}", output::ube(&format!("🧵 {name}"))).context("writing diff")?;
    } else {
        unified.header(name, name);
    }
    unified.to_writer(writer).context("writing diff")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use assert_matches::assert_matches;
    use rstest::rstest;
    use ruff_diagnostics::{Edit, Fix};
    use ruff_text_size::TextRange;
    use tempfile::TempDir;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::{Rule, RuleId};

    /// Test-only rule whose single edit rewrites the leading statement
    /// into unparseable source, exercising the reparse guard.
    struct BreaksParse;

    impl Rule for BreaksParse {
        fn apply(&self, _source: &Source) -> Vec<Vec<Edit>> {
            vec![vec![Edit::range_replacement(
                "def foo(".to_owned(),
                TextRange::new(0.into(), 5.into()),
            )]]
        }

        fn id(&self) -> RuleId {
            RuleId::from("breaks-parse")
        }

        fn message(&self) -> &'static str {
            "breaks parse"
        }
    }

    struct ErrorReader;

    impl Read for ErrorReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated stdin failure"))
        }
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated write failure"))
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

    fn diagnostic(severity: Severity, range: TextRange, slug: &'static str) -> Diagnostic {
        Diagnostic {
            fix: matches!(severity, Severity::Format)
                .then(|| Fix::safe_edit(Edit::range_replacement("y".into(), range))),
            message: "test".into(),
            range,
            rule: RuleId::from(slug),
            severity,
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

    fn outcome_with(source: Source, diagnostics: Vec<Diagnostic>) -> FileOutcome {
        FileOutcome::Done {
            cached: false,
            diagnostics,
            file: source.source_file().clone(),
            formatted_text: None,
        }
    }

    fn windowed() -> Presentation {
        Presentation {
            quiet: false,
            stdout_tty: false,
        }
    }

    #[test]
    fn cache_rewrite_marks_skipped_unless_run_supplied_text() {
        assert_matches!(cache_rewrite(false, None), Rewrite::Skipped);
        assert_matches!(cache_rewrite(true, None), Rewrite::Unchanged);
        assert_matches!(
            cache_rewrite(true, Some("y = 1\n")),
            Rewrite::Changed(text) if text == "y = 1\n"
        );
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
    fn check_outcomes_with_failed_parse_takes_higher_status() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let range = TextRange::new(0.into(), 1.into());
        let outcomes = vec![
            outcome_with(
                source,
                vec![diagnostic(Severity::Format, range, "synthetic-format")],
            ),
            FileOutcome::Failed(ExitStatus::ParseError),
        ];

        let status = status_from_outcomes(&outcomes, false);

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn check_outcomes_with_lint_and_format_returns_lint_violation() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let range = TextRange::new(0.into(), 1.into());
        let diagnostics = vec![
            diagnostic(Severity::Format, range, "synthetic-format"),
            diagnostic(Severity::Lint, range, "synthetic-lint"),
        ];
        let outcomes = vec![outcome_with(source, diagnostics)];

        let status = status_from_outcomes(&outcomes, false);

        assert_eq!(status, ExitStatus::LintViolation);
    }

    #[test]
    fn check_outcomes_with_synthetic_lint_returns_lint_violation() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let diagnostics = vec![diagnostic(
            Severity::Lint,
            TextRange::new(0.into(), 1.into()),
            "synthetic-lint",
        )];
        let outcomes = vec![outcome_with(source, diagnostics)];

        let status = status_from_outcomes(&outcomes, false);

        assert_eq!(status, ExitStatus::LintViolation);
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
    fn check_validate_fails_on_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(BreaksParse)]);
        let source = "x = 1\n".parse::<Source>().expect("parses");

        let outcome = run_pipeline(source, &pipeline, Pass::Diagnose { validate: true });

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn check_without_validate_ignores_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(BreaksParse)]);
        let source = "x = 1\n".parse::<Source>().expect("parses");

        let outcome = run_pipeline(source, &pipeline, Pass::Diagnose { validate: false });

        assert_matches!(outcome, FileOutcome::Done { .. });
    }

    #[test]
    fn emit_outcomes_propagates_writer_failure() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let diags = vec![diagnostic(
            Severity::Format,
            TextRange::new(0.into(), 1.into()),
            "synthetic-format",
        )];
        let outcomes = vec![outcome_with(source, diags)];
        let result = emit_outcomes(
            &outcomes,
            OutputFormat::Json,
            &mut FailingWriter,
            &EmitterSummary::default(),
        );
        assert!(result.is_err());
    }

    #[rstest]
    fn emit_outcomes_renders_each_output_format(
        #[values(
            OutputFormat::Github,
            OutputFormat::Json,
            OutputFormat::Sarif,
            OutputFormat::Text
        )]
        format: OutputFormat,
    ) {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let outcomes = vec![outcome_with(source, Vec::new())];
        let mut buf = Vec::new();
        emit_outcomes(&outcomes, format, &mut buf, &EmitterSummary::default()).expect("emits");
    }

    #[test]
    fn emitter_summary_counts_visited_changed_diagnostics_and_rules() {
        let range = TextRange::new(0.into(), 1.into());
        let mut changed = outcome_with(
            "x = 1\n".parse::<Source>().expect("parses"),
            vec![
                diagnostic(Severity::Format, range, "align-equals"),
                diagnostic(Severity::Lint, range, "reassigned-constants"),
            ],
        );
        if let FileOutcome::Done { formatted_text, .. } = &mut changed {
            *formatted_text = Some("x   = 1\n".to_owned());
        }
        let clean = outcome_with("y = 2\n".parse::<Source>().expect("parses"), Vec::new());
        let outcomes = vec![changed, clean, FileOutcome::Failed(ExitStatus::ParseError)];

        let summary = emitter_summary(&outcomes);

        assert_eq!(summary.files_visited, 2);
        assert_eq!(summary.files_changed, 1);
        assert_eq!(summary.files_with_diagnostics, 1);
        assert_eq!(summary.diagnostics_total, 2);
        assert_eq!(summary.rules_fired[&RuleId::from("align-equals")], 1);
        assert_eq!(
            summary.rules_fired[&RuleId::from("reassigned-constants")],
            1
        );
    }

    #[test]
    fn emitter_summary_tallies_repeated_rule_occurrences() {
        let range = TextRange::new(0.into(), 1.into());
        let outcome = outcome_with(
            "x = 1\n".parse::<Source>().expect("parses"),
            vec![
                diagnostic(Severity::Format, range, "align-equals"),
                diagnostic(Severity::Format, range, "align-equals"),
            ],
        );

        let summary = emitter_summary(std::slice::from_ref(&outcome));

        assert_eq!(summary.rules_fired[&RuleId::from("align-equals")], 2);
    }

    #[test]
    fn file_changed_counts_a_rewrite_or_a_format_diagnostic() {
        let range = TextRange::new(0.into(), 1.into());
        let format = vec![diagnostic(Severity::Format, range, "synthetic-format")];
        let lint = vec![diagnostic(Severity::Lint, range, "synthetic-lint")];

        assert!(file_changed(&[], Some("x = 1\n")));
        assert!(file_changed(&format, None));
        assert!(!file_changed(&lint, None));
        assert!(!file_changed(&[], None));
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

    #[test]
    fn process_path_returns_config_error_on_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let config = Config::default();
        let setup = RunSetup {
            cache: None,
            config_toml: String::new(),
            pipeline: Pipeline::with_filters(&config, &[], &[]),
        };
        let outcome = process_path(
            &tmp.path().join("does_not_exist.py"),
            &setup,
            Pass::Diagnose { validate: false },
        );
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn rehydrate_returns_none_for_a_skipped_entry() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Skipped,
        };
        assert!(rehydrate(Path::new("a.py"), b"x = 1\n", entry, true).is_none());
    }

    #[test]
    fn rehydrate_serves_a_changed_rewrite_to_a_format_mode() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Changed("y = 1\n".to_owned()),
        };
        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n", entry, true);
        assert_matches!(
            outcome,
            Some(FileOutcome::Done { formatted_text: Some(text), .. }) if text == "y = 1\n"
        );
    }

    #[test]
    fn rehydrate_serves_an_unchanged_rewrite_as_no_edit() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Unchanged,
        };
        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n", entry, true);
        assert_matches!(
            outcome,
            Some(FileOutcome::Done {
                formatted_text: None,
                ..
            })
        );
    }

    #[test]
    fn report_verbose_prints_bypassed_when_cache_disabled() {
        let mut buf = Vec::new();
        report_verbose(&[], false, &mut buf);
        assert_eq!(String::from_utf8(buf).expect("utf-8"), "cache: bypassed\n");
    }

    #[test]
    fn report_verbose_prints_hit_and_miss_counts() {
        let make = |cached: bool| {
            let source: Source = "x = 1\n".parse().expect("parses");
            let mut o = outcome_with(source, Vec::new());
            if let FileOutcome::Done { cached: c, .. } = &mut o {
                *c = cached;
            }
            o
        };
        let outcomes = vec![
            make(true),
            make(true),
            make(false),
            FileOutcome::Failed(ExitStatus::Clean),
        ];
        let mut buf = Vec::new();
        report_verbose(&outcomes, true, &mut buf);
        assert_eq!(
            String::from_utf8(buf).expect("utf-8"),
            "cache: 2 hits, 1 misses, 3 files\n",
        );
    }

    #[test]
    fn rewrite_pass_fails_on_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(BreaksParse)]);
        let source = "x = 1\n".parse::<Source>().expect("parses");

        let outcome = run_pipeline(source, &pipeline, Pass::Rewrite);

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn status_from_outcomes_demotes_format_change_when_demoted() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let outcomes = vec![outcome_with(
            source,
            vec![diagnostic(
                Severity::Format,
                TextRange::new(0.into(), 1.into()),
                "synthetic-format",
            )],
        )];
        assert_eq!(status_from_outcomes(&outcomes, true), ExitStatus::Clean);
        assert_eq!(
            status_from_outcomes(&outcomes, false),
            ExitStatus::FormatChange,
        );
    }

    #[test]
    fn summarize_reports_diagnostics_alongside_a_failure() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let outcomes = vec![
            outcome_with(
                source,
                vec![diagnostic(
                    Severity::Lint,
                    TextRange::new(0.into(), 1.into()),
                    "synthetic-lint",
                )],
            ),
            FileOutcome::Failed(ExitStatus::ParseError),
        ];
        assert_matches!(
            summarize(&outcomes, &emitter_summary(&outcomes), Mode::Check),
            Some(Summary::Diagnostics { files: 1, total: 1 })
        );
    }

    #[test]
    fn summarize_suppresses_clean_summary_when_a_file_failed() {
        let outcomes = vec![FileOutcome::Failed(ExitStatus::ParseError)];
        assert!(summarize(&outcomes, &emitter_summary(&outcomes), Mode::Check).is_none());
    }

    #[test]
    fn walk_error_returns_failed_with_config_error() {
        let outcome = walk_error("synthetic walk failure");
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn write_diff_decorates_with_thread_anchor() {
        let mut buf = Vec::new();
        {
            let mut writer = anstream::AutoStream::never(&mut buf);
            write_diff(
                &mut writer,
                "sample.py",
                "ab = 1\nx = 2\n",
                "ab = 1\nx  = 2\n",
                true,
            )
            .expect("writes");
        }
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(out.contains("🧵 sample.py"), "anchor missing: {out:?}");
        assert!(!out.contains("--- "), "plain header leaked: {out:?}");
        assert!(out.contains("@@"), "hunks missing: {out:?}");
    }

    #[test]
    fn write_diff_plain_keeps_the_patch_header() {
        let mut buf = Vec::new();
        write_diff(
            &mut buf,
            "sample.py",
            "ab = 1\nx = 2\n",
            "ab = 1\nx  = 2\n",
            false,
        )
        .expect("writes");
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(
            out.contains("--- sample.py"),
            "patch header missing: {out:?}"
        );
        assert!(!out.contains('🧵'), "decoration leaked: {out:?}");
    }
}

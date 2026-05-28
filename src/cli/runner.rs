//! Pipeline orchestration: load source, run, emit diagnostics, classify outcome.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use rayon::iter::{ParallelBridge, ParallelIterator};
use ruff_source_file::{SourceFile, SourceFileBuilder};

use super::args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter};
use super::exit_status::ExitStatus;
use crate::cache::{Cache, CacheEntry, CacheKey};
use crate::config::Config;
use crate::diagnostics::{Diagnostic, Emitter, Github, Json, Run, Sarif, Text};
use crate::pipeline::Pipeline;
use crate::source::Source;
use crate::walker;

/// One file's contribution to the run.
#[derive(Debug)]
enum FileOutcome {
    Done {
        cached: bool,
        diagnostics: Vec<Diagnostic>,
        file: SourceFile,
        formatted_text: Option<String>,
        original_text: String,
    },
    Failed(ExitStatus),
}

/// Per-run setup shared across every path the walker yields.
struct RunSetup {
    cache: Option<Cache>,
    config_toml: String,
    pipeline: Pipeline,
}

pub(crate) fn check_with_io<R: Read, W: Write>(
    args: CheckArgs,
    verbose: bool,
    stdin: R,
    mut stdout: W,
) -> anyhow::Result<ExitStatus> {
    let setup = match build_run(&args.rules, args.no_cache) {
        Ok(s) => s,
        Err(s) => return Ok(s),
    };
    let outcomes = if args.stdin {
        vec![process_stdin(stdin, &setup.pipeline)]
    } else {
        process_paths(&args.paths, |path| process_path(path, &setup))
    };
    emit_outcomes(&outcomes, args.output_format, &mut stdout)?;
    Ok(finish(outcomes, &setup, verbose, false))
}

pub(crate) fn format_with_io<R: Read, W: Write>(
    args: FormatArgs,
    verbose: bool,
    stdin: R,
    mut stdout: W,
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
            &setup.pipeline,
            &mut stdout,
        );
    }
    if args.diff {
        format_paths_diff(&args.paths, &setup, verbose, &mut stdout)
    } else {
        format_paths_rewrite(
            &args.paths,
            args.output_format,
            &setup,
            verbose,
            &mut stdout,
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
        eprintln!("error: {e}");
        return FileOutcome::Failed(ExitStatus::ConfigError);
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

fn emit_outcomes<W: Write>(
    outcomes: &[FileOutcome],
    format: OutputFormat,
    writer: &mut W,
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
        OutputFormat::Github => Github.emit(writer, &view),
        OutputFormat::Json => Json.emit(writer, &view),
        OutputFormat::Sarif => Sarif.emit(writer, &view),
        OutputFormat::Text => Text::new().emit(writer, &view),
    }?;
    writer.flush().context("flushing stdout")?;
    Ok(())
}

fn finish(
    outcomes: Vec<FileOutcome>,
    setup: &RunSetup,
    verbose: bool,
    demote_format_change: bool,
) -> ExitStatus {
    if verbose {
        report_verbose(&outcomes, setup.cache.is_some(), &mut io::stderr());
    }
    status_from_outcomes(&outcomes, demote_format_change)
}

fn format_paths_diff<W: Write>(
    paths: &[PathBuf],
    setup: &RunSetup,
    verbose: bool,
    stdout: &mut W,
) -> anyhow::Result<ExitStatus> {
    let outcomes = process_paths(paths, |path| process_path(path, setup));
    for outcome in &outcomes {
        if let FileOutcome::Done {
            file,
            formatted_text: Some(formatted),
            original_text,
            ..
        } = outcome
        {
            write_diff(stdout, file.name(), original_text, formatted)?;
        }
    }
    Ok(finish(outcomes, setup, verbose, false))
}

fn format_paths_rewrite<W: Write>(
    paths: &[PathBuf],
    format: OutputFormat,
    setup: &RunSetup,
    verbose: bool,
    stdout: &mut W,
) -> anyhow::Result<ExitStatus> {
    let outcomes = process_paths(paths, |path| apply_rewrite(path, process_path(path, setup)));
    if !format.is_text() {
        emit_outcomes(&outcomes, format, stdout)?;
    }
    Ok(finish(outcomes, setup, verbose, true))
}

fn format_stdin<R: Read, W: Write>(
    stdin: R,
    diff: bool,
    format: OutputFormat,
    pipeline: &Pipeline,
    writer: &mut W,
) -> anyhow::Result<ExitStatus> {
    let outcome = process_stdin(stdin, pipeline);
    if let FileOutcome::Done {
        file,
        formatted_text,
        original_text,
        ..
    } = &outcome
    {
        if diff {
            if let Some(formatted) = formatted_text {
                write_diff(writer, "<stdin>", original_text, formatted)?;
            }
        } else if format.is_text() {
            let to_write = formatted_text.as_deref().unwrap_or(file.source_text());
            writer
                .write_all(to_write.as_bytes())
                .context("writing stdout")?;
        } else {
            emit_outcomes(std::slice::from_ref(&outcome), format, writer)?;
        }
    }
    Ok(status_from_outcomes(std::slice::from_ref(&outcome), !diff))
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

fn process_path(path: &Path, setup: &RunSetup) -> FileOutcome {
    let bytes = match fs_err::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            return FileOutcome::Failed(ExitStatus::ConfigError);
        }
    };
    let cached = setup
        .cache
        .as_ref()
        .map(|c| (c, CacheKey::compute(&bytes, &setup.config_toml)));
    if let Some(outcome) = cached
        .as_ref()
        .and_then(|(c, k)| c.lookup(k))
        .and_then(|entry| rehydrate(path, &bytes, entry))
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
    let outcome = run_pipeline(source, &setup.pipeline);
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            formatted_text,
            ..
        },
    ) = (&cached, &outcome)
    {
        c.insert(
            k,
            &CacheEntry {
                diagnostics: diagnostics.clone(),
                formatted_source: formatted_text.clone(),
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

fn process_stdin<R: Read>(stdin: R, pipeline: &Pipeline) -> FileOutcome {
    let Ok(text) =
        io::read_to_string(stdin).inspect_err(|e| eprintln!("error: reading stdin: {e}"))
    else {
        return FileOutcome::Failed(ExitStatus::ConfigError);
    };
    text.parse::<Source>()
        .inspect_err(|e| eprintln!("error: parse error in stdin: {e}"))
        .map_or_else(
            |_| FileOutcome::Failed(ExitStatus::ParseError),
            |source| run_pipeline(source, pipeline),
        )
}

fn rehydrate(path: &Path, original_bytes: &[u8], entry: CacheEntry) -> Option<FileOutcome> {
    let original_text = std::str::from_utf8(original_bytes).ok()?.to_owned();
    let display_text = entry
        .formatted_source
        .as_deref()
        .unwrap_or(&original_text)
        .to_owned();
    let file = SourceFileBuilder::new(path.display().to_string(), display_text).finish();
    Some(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        formatted_text: entry.formatted_source,
        original_text,
    })
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

fn run_pipeline(source: Source, pipeline: &Pipeline) -> FileOutcome {
    let original_text = source.text().to_owned();
    match pipeline.run(source) {
        Ok((formatted, diagnostics)) => {
            let formatted_text =
                (formatted.text() != original_text).then(|| formatted.text().to_owned());
            FileOutcome::Done {
                cached: false,
                diagnostics,
                file: formatted.source_file().clone(),
                formatted_text,
                original_text,
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            FileOutcome::Failed(ExitStatus::ConfigError)
        }
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

fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    eprintln!("error: cannot walk: {err}");
    FileOutcome::Failed(ExitStatus::ConfigError)
}

/// Writes a unified diff between `before` and `after` to `writer`.
fn write_diff<W: Write>(
    writer: &mut W,
    name: impl std::fmt::Display,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    let header = name.to_string();
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .header(&header, &header)
        .to_writer(writer)
        .context("writing diff")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use assert_matches::assert_matches;
    use pretty_assertions::{assert_eq, assert_ne};
    use rstest::rstest;
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;
    use tempfile::TempDir;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::RuleId;

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
                .then(|| Edit::range_replacement("y".into(), range)),
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
        let original_text = source.text().to_owned();
        FileOutcome::Done {
            cached: false,
            diagnostics,
            file: source.source_file().clone(),
            formatted_text: None,
            original_text,
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
            io::empty(),
            Vec::<u8>::new(),
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
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn check_stdin_returns_clean_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let status = check_with_io(check_args(Vec::new(), true), false, stdin, Vec::<u8>::new())
            .expect("runs successfully");
        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn check_stdin_with_read_failure_returns_config_error() {
        let status = check_with_io(
            check_args(Vec::new(), true),
            false,
            ErrorReader,
            Vec::<u8>::new(),
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
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs without anyhow");

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn check_unparseable_stdin_returns_parse_error() {
        let stdin = Cursor::new(b"def foo(".to_vec());
        let status = check_with_io(check_args(Vec::new(), true), false, stdin, Vec::<u8>::new())
            .expect("runs without anyhow");
        assert_eq!(status, ExitStatus::ParseError);
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
        let result = emit_outcomes(&outcomes, OutputFormat::Json, &mut FailingWriter);
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
        emit_outcomes(&outcomes, format, &mut buf).expect("emits");
    }

    #[test]
    fn format_diff_returns_clean_for_already_canonical_file() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, true),
            false,
            io::empty(),
            Vec::<u8>::new(),
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
            io::empty(),
            Vec::<u8>::new(),
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
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn format_paths_does_not_rewrite_when_pipeline_is_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            false,
            io::empty(),
            Vec::<u8>::new(),
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
        let status =
            format_with_io(args, false, io::empty(), &mut stdout).expect("runs successfully");

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
            stdin,
            &mut stdout,
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
        let status = format_with_io(args, false, stdin, &mut stdout).expect("runs successfully");
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
            stdin,
            &mut stdout,
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
            ErrorReader,
            &mut stdout,
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
            io::empty(),
            Vec::<u8>::new(),
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
            io::empty(),
            Vec::<u8>::new(),
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
            io::empty(),
            Vec::<u8>::new(),
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
        let outcome = process_path(&tmp.path().join("does_not_exist.py"), &setup);
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
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
    fn walk_error_returns_failed_with_config_error() {
        let outcome = walk_error("synthetic walk failure");
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }
}

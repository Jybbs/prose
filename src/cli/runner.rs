//! Pipeline orchestration: load source, run, emit diagnostics, classify outcome.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use rayon::iter::{ParallelBridge, ParallelIterator};

use super::args::{CheckArgs, FormatArgs, OutputFormat, RuleFilter};
use super::exit_status::ExitStatus;
use super::log_error_chain;
use crate::config::Config;
use crate::diagnostics::{Diagnostic, Emitter, Github, Json, Run, Sarif, Text};
use crate::pipeline::Pipeline;
use crate::rule::RuleId;
use crate::source::{Source, SourceError};
use crate::walker;

/// One file's contribution to the run.
enum FileOutcome {
    Failed(ExitStatus),
    Parsed(Source, Vec<Diagnostic>),
}

pub(crate) fn check_with_io<R: Read, W: Write>(
    args: CheckArgs,
    stdin: R,
    mut stdout: W,
) -> anyhow::Result<ExitStatus> {
    let pipeline = match load_pipeline_or_status(&args.rules) {
        Ok(p) => p,
        Err(s) => return Ok(s),
    };
    let outcomes: Vec<FileOutcome> = if args.stdin {
        vec![process_stdin(stdin, &pipeline).1]
    } else {
        walker::walk(&args.paths)
            .par_bridge()
            .map(|entry| match entry {
                Ok(path) => process_path(&path, &pipeline),
                Err(e) => walk_error(e),
            })
            .collect()
    };
    emit_outcomes(&outcomes, args.output_format, &mut stdout)?;
    Ok(status_from_outcomes(&outcomes, false))
}

pub(crate) fn format_with_io<R: Read, W: Write>(
    args: FormatArgs,
    stdin: R,
    mut stdout: W,
) -> anyhow::Result<ExitStatus> {
    let pipeline = match load_pipeline_or_status(&args.rules) {
        Ok(p) => p,
        Err(s) => return Ok(s),
    };
    if args.stdin {
        return format_stdin(stdin, args.diff, args.output_format, &pipeline, &mut stdout);
    }
    if args.diff {
        format_paths_diff(&args.paths, args.output_format, &pipeline, &mut stdout)
    } else {
        format_paths_rewrite(&args.paths, args.output_format, &pipeline, &mut stdout)
    }
}

fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    match outcome {
        FileOutcome::Parsed(formatted, diagnostics) if !diagnostics.is_empty() => {
            match fs_err::write(path, formatted.text()).inspect_err(|e| eprintln!("error: {e}")) {
                Ok(()) => FileOutcome::Parsed(formatted, diagnostics),
                Err(_) => FileOutcome::Failed(ExitStatus::ConfigError),
            }
        }
        other => other,
    }
}

fn emit_outcomes<W: Write>(
    outcomes: &[FileOutcome],
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<()> {
    let view: Vec<Run<'_>> = outcomes
        .iter()
        .filter_map(|o| match o {
            FileOutcome::Failed(_) => None,
            FileOutcome::Parsed(s, d) => Some((s, d.as_slice())),
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

fn format_paths_diff<W: Write>(
    paths: &[PathBuf],
    format: OutputFormat,
    pipeline: &Pipeline,
    stdout: &mut W,
) -> anyhow::Result<ExitStatus> {
    let entries: Vec<(PathBuf, String, FileOutcome)> = walker::walk(paths)
        .par_bridge()
        .map(|entry| match entry {
            Ok(path) => {
                let (original, outcome) = process_path_with_original(&path, pipeline);
                (path, original, outcome)
            }
            Err(e) => (PathBuf::new(), String::new(), walk_error(e)),
        })
        .collect();
    let outcomes: Vec<FileOutcome> = entries
        .into_iter()
        .map(|(path, original, outcome)| -> anyhow::Result<FileOutcome> {
            if let FileOutcome::Parsed(formatted, diags) = &outcome {
                if !diags.is_empty() {
                    write_diff(stdout, path.display(), &original, formatted.text())?;
                }
            }
            Ok(outcome)
        })
        .collect::<anyhow::Result<_>>()?;
    if !format.is_text() {
        emit_outcomes(&outcomes, format, stdout)?;
    }
    Ok(status_from_outcomes(&outcomes, false))
}

fn format_paths_rewrite<W: Write>(
    paths: &[PathBuf],
    format: OutputFormat,
    pipeline: &Pipeline,
    stdout: &mut W,
) -> anyhow::Result<ExitStatus> {
    let outcomes: Vec<FileOutcome> = walker::walk(paths)
        .par_bridge()
        .map(|entry| match entry {
            Ok(path) => apply_rewrite(&path, process_path(&path, pipeline)),
            Err(e) => walk_error(e),
        })
        .collect();
    if !format.is_text() {
        emit_outcomes(&outcomes, format, stdout)?;
    }
    Ok(status_from_outcomes(&outcomes, true))
}

fn format_stdin<R: Read, W: Write>(
    stdin: R,
    diff: bool,
    format: OutputFormat,
    pipeline: &Pipeline,
    writer: &mut W,
) -> anyhow::Result<ExitStatus> {
    let (original, outcome) = process_stdin(stdin, pipeline);
    let (formatted, diagnostics) = match outcome {
        FileOutcome::Failed(s) => return Ok(s),
        FileOutcome::Parsed(s, d) => (s, d),
    };

    if diff && !diagnostics.is_empty() {
        write_diff(writer, "<stdin>", &original, formatted.text())?;
    } else if !diff && format.is_text() {
        writer
            .write_all(formatted.text().as_bytes())
            .context("writing stdout")?;
    }
    let outcomes = vec![FileOutcome::Parsed(formatted, diagnostics)];
    if !diff && !format.is_text() {
        emit_outcomes(&outcomes, format, writer)?;
    }
    Ok(status_from_outcomes(&outcomes, !diff))
}

fn load_pipeline(select: &[RuleId], ignore: &[RuleId]) -> anyhow::Result<Pipeline> {
    let cwd = std::env::current_dir().context("reading current working directory")?;
    let config = Config::load(&cwd).context("loading [tool.prose] config")?;
    Ok(Pipeline::with_filters(&config, select, ignore))
}

fn load_pipeline_or_status(filter: &RuleFilter) -> Result<Pipeline, ExitStatus> {
    load_pipeline(&filter.select, &filter.ignore)
        .inspect_err(log_error_chain)
        .map_err(|_| ExitStatus::ConfigError)
}

fn process_path(path: &Path, pipeline: &Pipeline) -> FileOutcome {
    match read_source_or_status(path) {
        Ok(source) => run_pipeline(source, pipeline),
        Err(s) => FileOutcome::Failed(s),
    }
}

fn process_path_with_original(path: &Path, pipeline: &Pipeline) -> (String, FileOutcome) {
    match read_source_or_status(path) {
        Ok(source) => {
            let original = source.text().to_owned();
            (original, run_pipeline(source, pipeline))
        }
        Err(s) => (String::new(), FileOutcome::Failed(s)),
    }
}

fn process_stdin<R: Read>(stdin: R, pipeline: &Pipeline) -> (String, FileOutcome) {
    let text =
        match io::read_to_string(stdin).inspect_err(|e| eprintln!("error: reading stdin: {e}")) {
            Ok(text) => text,
            Err(_) => return (String::new(), FileOutcome::Failed(ExitStatus::ConfigError)),
        };
    let outcome = match text
        .parse::<Source>()
        .inspect_err(|e| eprintln!("error: parse error in stdin: {e}"))
    {
        Ok(source) => run_pipeline(source, pipeline),
        Err(_) => FileOutcome::Failed(ExitStatus::ParseError),
    };
    (text, outcome)
}

fn read_source_or_status(path: &Path) -> Result<Source, ExitStatus> {
    Source::from_path(path).map_err(|e| match e {
        SourceError::Io(io_err) => {
            eprintln!("error: {io_err}");
            ExitStatus::ConfigError
        }
        SourceError::Parse(parse_err) => {
            eprintln!("error: parse error in `{}`: {parse_err}", path.display());
            ExitStatus::ParseError
        }
    })
}

fn run_pipeline(source: Source, pipeline: &Pipeline) -> FileOutcome {
    match pipeline
        .run(source)
        .inspect_err(|e| eprintln!("error: {e}"))
    {
        Ok((s, d)) => FileOutcome::Parsed(s, d),
        Err(_) => FileOutcome::Failed(ExitStatus::ConfigError),
    }
}

fn status_from_outcomes(outcomes: &[FileOutcome], demote_format_change: bool) -> ExitStatus {
    outcomes
        .iter()
        .map(|outcome| match outcome {
            FileOutcome::Failed(s) => *s,
            FileOutcome::Parsed(_, diags) => diags
                .iter()
                .map(|d| ExitStatus::from(d.severity))
                .filter(|s| !demote_format_change || *s != ExitStatus::FormatChange)
                .max()
                .unwrap_or_default(),
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

    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;
    use tempfile::TempDir;

    use super::*;
    use crate::diagnostics::Severity;

    fn check_args(paths: Vec<PathBuf>, stdin: bool) -> CheckArgs {
        CheckArgs {
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
            paths,
            stdin,
            ..Default::default()
        }
    }

    #[test]
    fn check_clean_returns_clean() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let status = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
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
            FileOutcome::Parsed(
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
        let outcomes = vec![FileOutcome::Parsed(source, diagnostics)];

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
        let outcomes = vec![FileOutcome::Parsed(source, diagnostics)];

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
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::FormatChange);
    }

    #[test]
    fn check_stdin_returns_clean_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let status = check_with_io(check_args(Vec::new(), true), stdin, Vec::<u8>::new())
            .expect("runs successfully");
        assert_eq!(status, ExitStatus::Clean);
    }

    #[test]
    fn check_unparseable_path_returns_parse_error() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "def foo(").expect("writes");

        let status = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs without anyhow");

        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn check_unparseable_stdin_returns_parse_error() {
        let stdin = Cursor::new(b"def foo(".to_vec());
        let status = check_with_io(check_args(Vec::new(), true), stdin, Vec::<u8>::new())
            .expect("runs without anyhow");
        assert_eq!(status, ExitStatus::ParseError);
    }

    #[test]
    fn format_diff_returns_format_change_for_pending_change() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "alpha = 1\nb = 22\n").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, true),
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
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
        let contents = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(contents, "x = 1\n");
    }

    #[test]
    fn format_stdin_prints_input_verbatim_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let mut stdout = Vec::new();
        let status = format_with_io(format_args(Vec::new(), true, false), stdin, &mut stdout)
            .expect("runs successfully");
        assert_eq!(status, ExitStatus::Clean);
        assert_eq!(stdout, b"x = 1\n");
    }

    #[test]
    fn format_unparseable_returns_parse_error() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "def foo(").expect("writes");

        let status = format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
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
            io::empty(),
            Vec::<u8>::new(),
        )
        .expect("runs successfully");

        assert_eq!(status, ExitStatus::Clean);
        let after = std::fs::read_to_string(&file).expect("reads");
        assert_ne!(after, "alpha = 1\nb = 22\n");
    }

    #[test]
    fn status_from_outcomes_demotes_format_change_when_demoted() {
        let source = "x = 1\n".parse::<Source>().expect("parses");
        let outcomes = vec![FileOutcome::Parsed(
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
}

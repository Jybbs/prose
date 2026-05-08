//! Pipeline orchestration: load source, run, emit diagnostics.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use rayon::iter::{ParallelBridge, ParallelIterator};

use super::args::{CheckArgs, FormatArgs, OutputFormat};
use crate::config::Config;
use crate::diagnostics::{Diagnostic, Emitter, Github, Json, Run, Sarif, Text};
use crate::pipeline::Pipeline;
use crate::rule::RuleId;
use crate::source::Source;
use crate::walker;

/// Returns `true` when no diagnostics fire, `false` when at least
/// one rule would rewrite the input.
pub(crate) fn check_with_io<R: Read, W: Write>(
    args: CheckArgs,
    stdin: R,
    mut stdout: W,
) -> anyhow::Result<bool> {
    let pipeline = load_pipeline(&args.rules.select, &args.rules.ignore)?;
    let emitter = build_emitter(args.output_format);

    let runs: Vec<(Source, Vec<Diagnostic>)> = if args.stdin {
        let (_, source, diagnostics) = run_pipeline_on_stdin(stdin, &pipeline)?;
        vec![(source, diagnostics)]
    } else {
        walker::walk(&args.paths)
            .par_bridge()
            .map(|entry| {
                let path = entry.context("walking input paths")?;
                let (_, source, diagnostics) = process_path(&path, &pipeline)?;
                Ok((source, diagnostics))
            })
            .collect::<anyhow::Result<_>>()?
    };

    let view: Vec<Run<'_>> = runs.iter().map(|(s, d)| (s, d.as_slice())).collect();
    emitter.emit(&mut stdout, &view)?;
    stdout.flush().context("flushing stdout")?;
    Ok(runs.iter().all(|(_, d)| d.is_empty()))
}

pub(crate) fn format_with_io<R: Read, W: Write>(
    args: FormatArgs,
    stdin: R,
    mut stdout: W,
) -> anyhow::Result<()> {
    let pipeline = load_pipeline(&args.rules.select, &args.rules.ignore)?;

    if args.stdin {
        let (original, formatted, diagnostics) = run_pipeline_on_stdin(stdin, &pipeline)?;
        if args.diff {
            if !diagnostics.is_empty() {
                write_diff(&mut stdout, "<stdin>", &original, formatted.text())?;
            }
        } else if matches!(args.output_format, OutputFormat::Text) {
            stdout
                .write_all(formatted.text().as_bytes())
                .context("writing stdout")?;
        } else {
            let emitter = build_emitter(args.output_format);
            emitter.emit(&mut stdout, &[(&formatted, &diagnostics)])?;
            stdout.flush().context("flushing stdout")?;
        }
        return Ok(());
    }

    let runs: Vec<(PathBuf, String, Source, Vec<Diagnostic>)> = walker::walk(&args.paths)
        .par_bridge()
        .map(
            |entry| -> anyhow::Result<(PathBuf, String, Source, Vec<Diagnostic>)> {
                let path = entry.context("walking input paths")?;
                let (original, formatted, diagnostics) = process_path(&path, &pipeline)?;
                if !diagnostics.is_empty() && !args.diff {
                    fs_err::write(&path, formatted.text())?;
                }
                Ok((path, original, formatted, diagnostics))
            },
        )
        .collect::<anyhow::Result<Vec<_>>>()?;

    if args.diff {
        for (path, original, formatted, diagnostics) in &runs {
            if !diagnostics.is_empty() {
                write_diff(&mut stdout, path.display(), original, formatted.text())?;
            }
        }
    } else if !matches!(args.output_format, OutputFormat::Text) {
        let emitter = build_emitter(args.output_format);
        let view: Vec<Run<'_>> = runs.iter().map(|(_, _, s, d)| (s, d.as_slice())).collect();
        emitter.emit(&mut stdout, &view)?;
        stdout.flush().context("flushing stdout")?;
    }
    Ok(())
}

fn build_emitter(format: OutputFormat) -> Box<dyn Emitter> {
    match format {
        OutputFormat::Github => Box::new(Github),
        OutputFormat::Json => Box::new(Json),
        OutputFormat::Sarif => Box::new(Sarif),
        OutputFormat::Text => Box::new(Text::new()),
    }
}

fn load_pipeline(select: &[RuleId], ignore: &[RuleId]) -> anyhow::Result<Pipeline> {
    let cwd = std::env::current_dir().context("reading current working directory")?;
    let config = Config::load(&cwd).context("loading [tool.prose] config")?;
    Ok(Pipeline::with_filters(&config, select, ignore))
}

/// Loads a single Python file from disk, captures its original text,
/// and runs the pipeline over it.
fn process_path(
    path: &Path,
    pipeline: &Pipeline,
) -> anyhow::Result<(String, Source, Vec<Diagnostic>)> {
    let source =
        Source::from_path(path).with_context(|| format!("loading `{}`", path.display()))?;
    let original = source.text().to_owned();
    let (formatted, diagnostics) = pipeline.run(source)?;
    Ok((original, formatted, diagnostics))
}

/// Reads a Python source from `stdin`, parses it, runs the pipeline,
/// and returns the captured original text alongside the result.
fn run_pipeline_on_stdin<R: Read>(
    stdin: R,
    pipeline: &Pipeline,
) -> anyhow::Result<(String, Source, Vec<Diagnostic>)> {
    let text = io::read_to_string(stdin).context("reading stdin")?;
    let source: Source = text.parse().context("parsing stdin as Python")?;
    let (formatted, diagnostics) = pipeline.run(source)?;
    Ok((text, formatted, diagnostics))
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
    use std::io::Cursor;

    use tempfile::TempDir;

    use super::super::args::RuleFilter;
    use super::*;

    fn check_args(paths: Vec<PathBuf>, stdin: bool) -> CheckArgs {
        CheckArgs {
            output_format: OutputFormat::default(),
            paths,
            rules: RuleFilter::default(),
            stdin,
        }
    }

    fn format_args(paths: Vec<PathBuf>, stdin: bool, diff: bool) -> FormatArgs {
        FormatArgs {
            diff,
            output_format: OutputFormat::default(),
            paths,
            rules: RuleFilter::default(),
            stdin,
        }
    }

    #[test]
    fn check_paths_reports_clean_when_pipeline_is_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let clean = check_with_io(
            check_args(vec![tmp.path().to_path_buf()], false),
            Cursor::new(Vec::<u8>::new()),
            Vec::<u8>::new(),
        )
        .expect("runs successfully");

        assert!(clean);
    }

    #[test]
    fn check_stdin_empty_pipeline_reports_clean() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let clean = check_with_io(check_args(Vec::new(), true), stdin, Vec::<u8>::new())
            .expect("runs successfully");
        assert!(clean);
    }

    #[test]
    fn check_stdin_surfaces_parse_error() {
        let stdin = Cursor::new(b"def foo(".to_vec());
        let err = check_with_io(check_args(Vec::new(), true), stdin, Vec::<u8>::new()).unwrap_err();
        assert!(err.to_string().contains("stdin"));
    }

    #[test]
    fn format_paths_does_not_rewrite_when_pipeline_is_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").expect("writes");

        let mut stdout = Vec::new();
        format_with_io(
            format_args(vec![tmp.path().to_path_buf()], false, false),
            Cursor::new(Vec::<u8>::new()),
            &mut stdout,
        )
        .expect("runs successfully");

        let contents = std::fs::read_to_string(&file).expect("reads");
        assert_eq!(contents, "x = 1\n");
        assert!(stdout.is_empty());
    }

    #[test]
    fn format_stdin_prints_input_verbatim_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let mut stdout = Vec::new();
        format_with_io(format_args(Vec::new(), true, false), stdin, &mut stdout)
            .expect("runs successfully");
        assert_eq!(stdout, b"x = 1\n");
    }

    #[test]
    fn format_stdin_with_diff_emits_nothing_when_pipeline_is_empty() {
        let stdin = Cursor::new(b"x = 1\n".to_vec());
        let mut stdout = Vec::new();
        format_with_io(format_args(Vec::new(), true, true), stdin, &mut stdout)
            .expect("runs successfully");
        assert!(stdout.is_empty());
    }
}

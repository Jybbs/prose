//! Per-file processing: read, resolve config, cache-lookup, run the
//! pipeline, and classify the outcome.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use rayon::iter::{ParallelBridge, ParallelIterator};
use ruff_notebook::NotebookIndex;
use ruff_python_ast::PySourceType;
use ruff_source_file::{SourceFile, SourceFileBuilder};

use super::{FileOutcome, Pass, RunSetup, has_format_change, notebook};
use crate::{
    cache::{CacheEntry, CacheKey, Rewrite},
    cli::exit_status::ExitStatus,
    pipeline::Pipeline,
    source::Source,
    walker,
};

pub(super) fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    let FileOutcome::Done {
        rewrite: Rewrite::Changed(kind),
        ..
    } = &outcome
    else {
        return outcome;
    };
    if let Err(e) = fs_err::write(path, kind.written()) {
        return failed(ExitStatus::ConfigError, e);
    }
    outcome
}

pub(super) fn process_path(
    path: &Path,
    source_type: PySourceType,
    setup: &RunSetup,
    pass: Pass,
) -> FileOutcome {
    let bytes = match fs_err::read(path) {
        Ok(b) => b,
        Err(e) => return failed(ExitStatus::ConfigError, e),
    };
    let Some(resolved) = setup.resolver.resolve(path, &bytes) else {
        return FileOutcome::Failed(ExitStatus::ConfigError);
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
        .map(|c| {
            (
                c,
                CacheKey::compute(&bytes, &resolved.config_toml, resolved.pipeline.rule_ids()),
            )
        });
    if let Some(outcome) = keyed
        .as_ref()
        .and_then(|(c, k)| c.lookup(k))
        .and_then(|entry| rehydrate(path, source_type, &bytes, entry, needs_rewrite))
    {
        return outcome;
    }
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            return failed(
                ExitStatus::ConfigError,
                format_args!("{} is not valid UTF-8: {e}", path.display()),
            );
        }
    };
    let outcome = if source_type.is_ipynb() {
        notebook::process(text, path.display().to_string(), &resolved.pipeline, pass)
    } else {
        match Source::build_module(text, path.display().to_string(), source_type) {
            Ok(source) => run_pipeline(source, &resolved.pipeline, pass),
            Err(e) => failed(
                ExitStatus::ParseError,
                format_args!("parse error in `{}`: {e}", path.display()),
            ),
        }
    };
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            rewrite,
            ..
        },
    ) = (&keyed, &outcome)
    {
        c.insert(
            k,
            &CacheEntry {
                diagnostics: diagnostics.clone(),
                rewrite: rewrite.clone(),
            },
        );
    }
    outcome
}

pub(super) fn process_paths<F>(paths: &[PathBuf], handle: F) -> Vec<FileOutcome>
where
    F: Fn(&Path, PySourceType) -> FileOutcome + Send + Sync,
{
    walker::walk(paths)
        .par_bridge()
        .map(|entry| {
            entry.map_or_else(walk_error, |(path, source_type)| handle(&path, source_type))
        })
        .collect()
}

pub(super) fn process_stdin(
    text: String,
    source_type: PySourceType,
    pipeline: &Pipeline,
    pass: Pass,
) -> FileOutcome {
    if source_type.is_ipynb() {
        return notebook::process(text, "<stdin>".to_owned(), pipeline, pass);
    }
    match Source::build_module(text, "<stdin>", source_type) {
        Ok(source) => run_pipeline(source, pipeline, pass),
        Err(e) => failed(
            ExitStatus::ParseError,
            format_args!("parse error in stdin: {e}"),
        ),
    }
}

/// Reads stdin to a string, mapping a read failure to a config-error
/// outcome.
pub(super) fn read_stdin<R: Read>(stdin: R) -> Result<String, FileOutcome> {
    io::read_to_string(stdin)
        .map_err(|e| failed(ExitStatus::ConfigError, format_args!("reading stdin: {e}")))
}

pub(super) fn rehydrate(
    path: &Path,
    source_type: PySourceType,
    original_bytes: &[u8],
    entry: CacheEntry,
    needs_rewrite: bool,
) -> Option<FileOutcome> {
    let rewrite = if needs_rewrite {
        match entry.rewrite {
            // A `check` entry skipped the rewrite this mode needs.
            Rewrite::Skipped => return None,
            rewrite => rewrite,
        }
    } else {
        Rewrite::Skipped
    };
    let text = std::str::from_utf8(original_bytes).ok()?;
    let (source_text, notebook_index) = if source_type.is_ipynb() {
        let (code, index) = notebook::rehydrated(text)?;
        (code, Some(index))
    } else {
        (text.to_owned(), None)
    };
    let file = SourceFileBuilder::new(path.display().to_string(), source_text).finish();
    Some(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        notebook_index: notebook_index.map(Box::new),
        rewrite,
    })
}

/// Runs a text source through the pipeline. A check pass collects the
/// as-written diagnostics through [`diagnose_only`]; a format pass
/// builds the text rewrite through [`run_and_assemble`]. A module carries
/// no notebook index.
pub(super) fn run_pipeline(source: Source, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    if let Pass::Diagnose { validate } = pass {
        return diagnose_only(source, pipeline, validate, None);
    }
    run_and_assemble(
        source,
        pipeline,
        matches!(pass, Pass::Both),
        None,
        |formatted, file| {
            formatted
                .changed_from(file.source_text())
                .map_or(Rewrite::Unchanged, |text| Rewrite::text(text.to_owned()))
        },
    )
}

/// Runs the pipeline and assembles the outcome, deferring the rewrite
/// to `rewrite`. The caller handles the diagnose-only pass, while the
/// `diagnose_as_written` flag adds the as-written diagnostics an output
/// format renders beside the rewrite.
pub(super) fn run_and_assemble(
    source: Source,
    pipeline: &Pipeline,
    diagnose_as_written: bool,
    notebook_index: Option<NotebookIndex>,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    let file = source.source_file().clone();
    let diagnosed = diagnose_as_written.then(|| pipeline.diagnose(&source));
    match pipeline.run(source) {
        Ok((formatted, run_diagnostics)) => {
            let rewrite = rewrite(&formatted, &file);
            FileOutcome::Done {
                cached: false,
                diagnostics: diagnosed.unwrap_or(run_diagnostics),
                file,
                notebook_index: notebook_index.map(Box::new),
                rewrite,
            }
        }
        Err(e) => failed(ExitStatus::ConfigError, e),
    }
}

/// Collects the as-written diagnostics, and with `validate` guards the
/// would-be rewrite against an unparseable output. Shared by the
/// module and notebook check passes.
pub(super) fn diagnose_only(
    source: Source,
    pipeline: &Pipeline,
    validate: bool,
    notebook_index: Option<NotebookIndex>,
) -> FileOutcome {
    let file = source.source_file().clone();
    let diagnostics = pipeline.diagnose(&source);
    if validate
        && has_format_change(&diagnostics)
        && let Err(e) = pipeline.validate(source)
    {
        return failed(ExitStatus::ConfigError, e);
    }
    FileOutcome::Done {
        cached: false,
        diagnostics,
        file,
        notebook_index: notebook_index.map(Box::new),
        rewrite: Rewrite::Skipped,
    }
}

pub(super) fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    failed(ExitStatus::ConfigError, format_args!("cannot walk: {err}"))
}

pub(super) fn failed(status: ExitStatus, e: impl std::fmt::Display) -> FileOutcome {
    eprintln!("error: {e}");
    FileOutcome::Failed(status)
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use ruff_diagnostics::Edit;
    use tempfile::TempDir;

    use super::super::report::status_from_outcomes;
    use super::super::resolve::ConfigResolver;
    use super::*;
    use crate::cache::RewriteKind;
    use crate::config::Config;
    use crate::rule::RuleId;
    use crate::testing::{GroupSentinelRule, breaks_parse, parse, range};

    #[test]
    fn check_validate_fails_on_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &pipeline, Pass::Diagnose { validate: true });

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn check_without_validate_ignores_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &pipeline, Pass::Diagnose { validate: false });

        assert_matches!(
            outcome,
            FileOutcome::Done {
                rewrite: Rewrite::Skipped,
                ..
            }
        );
    }

    #[test]
    fn process_path_returns_config_error_on_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let resolver = ConfigResolver::new(Vec::new(), Vec::new());
        let cwd = resolver.seed(&Config::default());
        let setup = RunSetup {
            cache: None,
            cwd,
            resolver,
        };
        let outcome = process_path(
            &tmp.path().join("does_not_exist.py"),
            PySourceType::Python,
            &setup,
            Pass::Diagnose { validate: false },
        );
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn rehydrate_marks_a_check_mode_outcome_skipped() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::text("y = 1\n".to_owned()),
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n",
            entry,
            false,
        );
        assert_matches!(
            outcome,
            Some(FileOutcome::Done {
                rewrite: Rewrite::Skipped,
                ..
            })
        );
    }

    #[test]
    fn rehydrate_returns_none_for_a_skipped_entry() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Skipped,
        };
        assert!(
            rehydrate(
                Path::new("a.py"),
                PySourceType::Python,
                b"x = 1\n",
                entry,
                true
            )
            .is_none()
        );
    }

    #[test]
    fn rehydrate_serves_a_changed_rewrite_to_a_format_mode() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::text("y = 1\n".to_owned()),
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n",
            entry,
            true,
        );
        assert_matches!(
            outcome,
            Some(FileOutcome::Done { rewrite: Rewrite::Changed(RewriteKind::Text(text)), .. })
                if text == "y = 1\n"
        );
    }

    #[test]
    fn rehydrate_serves_an_unchanged_rewrite_as_no_edit() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Unchanged,
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n",
            entry,
            true,
        );
        assert_matches!(
            outcome,
            Some(FileOutcome::Done {
                rewrite: Rewrite::Unchanged,
                ..
            })
        );
    }

    #[test]
    fn rewrite_pass_fails_on_unparseable_rule_output() {
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &pipeline, Pass::Rewrite);

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn run_pipeline_reports_unchanged_when_edits_cancel() {
        let range = range(0, 1);
        let pipeline = Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("y".to_owned(), range)]],
                id: RuleId::from("x-to-y"),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("x".to_owned(), range)]],
                id: RuleId::from("y-to-x"),
            }),
        ]);
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &pipeline, Pass::Rewrite);

        assert_matches!(
            &outcome,
            FileOutcome::Done {
                diagnostics,
                rewrite: Rewrite::Unchanged,
                ..
            } if diagnostics.len() == 2
        );
        assert_eq!(
            status_from_outcomes(std::slice::from_ref(&outcome), false),
            ExitStatus::Clean,
        );
    }

    #[test]
    fn walk_error_returns_failed_with_config_error() {
        let outcome = walk_error("synthetic walk failure");
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }
}

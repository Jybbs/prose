//! Per-file processing: read, resolve config, cache-lookup, run the
//! pipeline, and classify the outcome.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use rayon::iter::{ParallelBridge, ParallelIterator};
use ruff_source_file::SourceFileBuilder;

use super::{FileOutcome, Pass, RunSetup, has_format_change};
use crate::{
    cache::{CacheEntry, CacheKey, Rewrite},
    cli::exit_status::ExitStatus,
    pipeline::Pipeline,
    source::Source,
    walker,
};

pub(super) fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    let FileOutcome::Done {
        rewrite: Rewrite::Changed(text),
        ..
    } = &outcome
    else {
        return outcome;
    };
    if let Err(e) = fs_err::write(path, text) {
        return failed(ExitStatus::ConfigError, e);
    }
    outcome
}

pub(super) fn process_path(path: &Path, setup: &RunSetup, pass: Pass) -> FileOutcome {
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
        .map(|c| (c, CacheKey::compute(&bytes, &resolved.config_toml)));
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
            return failed(
                ExitStatus::ConfigError,
                format_args!("{} is not valid UTF-8: {e}", path.display()),
            );
        }
    };
    let source = match Source::build(text, path.display().to_string()) {
        Ok(s) => s,
        Err(e) => {
            return failed(
                ExitStatus::ParseError,
                format_args!("parse error in `{}`: {e}", path.display()),
            );
        }
    };
    let outcome = run_pipeline(source, &resolved.pipeline, pass);
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
    F: Fn(&Path) -> FileOutcome + Send + Sync,
{
    walker::walk(paths)
        .par_bridge()
        .map(|entry| entry.map_or_else(walk_error, |path| handle(&path)))
        .collect()
}

pub(super) fn process_stdin<R: Read>(stdin: R, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let text = match io::read_to_string(stdin) {
        Ok(t) => t,
        Err(e) => return failed(ExitStatus::ConfigError, format_args!("reading stdin: {e}")),
    };
    match text.parse::<Source>() {
        Ok(source) => run_pipeline(source, pipeline, pass),
        Err(e) => failed(
            ExitStatus::ParseError,
            format_args!("parse error in stdin: {e}"),
        ),
    }
}

pub(super) fn rehydrate(
    path: &Path,
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
    let original_text = std::str::from_utf8(original_bytes).ok()?.to_owned();
    let file = SourceFileBuilder::new(path.display().to_string(), original_text).finish();
    Some(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        rewrite,
    })
}

/// Computes only the passes `pass` reads. `check` collects the
/// as-written diagnostics, and with `--validate` set it also guards the
/// would-be rewrite against an unparseable rule output without rebuilding
/// diagnostics. A `format` run rewrites through `run`, pairing the
/// rewrite with `diagnose`'s as-written diagnostics when an output format
/// will render them, or `run`'s own otherwise.
pub(super) fn run_pipeline(source: Source, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let file = source.source_file().clone();
    if let Pass::Diagnose { validate } = pass {
        let diagnostics = pipeline.diagnose(&source);
        if validate
            && has_format_change(&diagnostics)
            && let Err(e) = pipeline.validate(source)
        {
            return failed(ExitStatus::ConfigError, e);
        }
        return FileOutcome::Done {
            cached: false,
            diagnostics,
            file,
            rewrite: Rewrite::Skipped,
        };
    }
    let diagnosed = matches!(pass, Pass::Both).then(|| pipeline.diagnose(&source));
    match pipeline.run(source) {
        Ok((formatted, run_diagnostics)) => {
            let rewrite = formatted
                .changed_from(file.source_text())
                .map_or(Rewrite::Unchanged, |text| Rewrite::Changed(text.to_owned()));
            FileOutcome::Done {
                cached: false,
                diagnostics: diagnosed.unwrap_or(run_diagnostics),
                file,
                rewrite,
            }
        }
        Err(e) => failed(ExitStatus::ConfigError, e),
    }
}

pub(super) fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    failed(ExitStatus::ConfigError, format_args!("cannot walk: {err}"))
}

fn failed(status: ExitStatus, e: impl std::fmt::Display) -> FileOutcome {
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
            &setup,
            Pass::Diagnose { validate: false },
        );
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn rehydrate_marks_a_check_mode_outcome_skipped() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Changed("y = 1\n".to_owned()),
        };
        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n", entry, false);
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
            Some(FileOutcome::Done { rewrite: Rewrite::Changed(text), .. }) if text == "y = 1\n"
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

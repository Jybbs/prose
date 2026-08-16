//! Per-file processing: read, resolve config, cache-lookup, run the
//! pipeline, and classify the outcome.

use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use rayon::iter::{ParallelBridge, ParallelIterator};
use ruff_notebook::NotebookIndex;
use ruff_python_ast::PySourceType;
use ruff_source_file::{SourceFile, SourceFileBuilder};
use tempfile::NamedTempFile;

use super::{
    FileOutcome, Pass, RunSetup, STDIN_NAME, has_format_change, notebook, resolve::Resolved,
};
use crate::{
    cache::{CacheEntry, CacheKey, Rewrite},
    cli::exit_status::ExitStatus,
    source::Source,
    unstable::UnstableRewrite,
    walker::{self, Found},
};

pub(super) fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    let FileOutcome::Done {
        rewrite: Rewrite::Changed(kind),
        ..
    } = &outcome
    else {
        return outcome;
    };
    if let Err(e) = write_atomic(path, kind.written()) {
        return failed(ExitStatus::ConfigError, e);
    }
    outcome
}

/// Dispatches `source` by `pass`, collecting the as-written diagnostics on
/// a check pass and building the rewrite through `rewrite` on a format
/// pass. A notebook threads its `index`, a module passes `None`.
pub(super) fn drive(
    source: Source,
    resolved: &Resolved,
    pass: Pass,
    index: Option<NotebookIndex>,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    if let Pass::Diagnose { validate } = pass {
        return diagnose_only(source, resolved, validate, index);
    }
    run_and_assemble(source, resolved, matches!(pass, Pass::Both), index, rewrite)
}

pub(super) fn failed(status: ExitStatus, e: impl std::fmt::Display) -> FileOutcome {
    eprintln!("error: {e}");
    FileOutcome::Failed(status)
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
    let outcome = process_source(
        text,
        path.display().to_string(),
        source_type,
        &resolved,
        pass,
    );
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            rewrite,
            unstable,
            ..
        },
    ) = (&keyed, &outcome)
    {
        c.insert(
            k,
            &CacheEntry {
                diagnostics: diagnostics.clone(),
                rewrite: rewrite.clone(),
                unstable: unstable.clone(),
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
        .filter_map(|entry| match entry {
            Ok(Found::Formattable(path, source_type)) => Some(handle(&path, source_type)),
            Ok(Found::PassedLink(path)) => {
                eprintln!("note: passed over the symlink {}", path.display());
                None
            }
            Err(e) => Some(walk_error(e)),
        })
        .collect()
}

pub(super) fn process_stdin(
    text: String,
    source_type: PySourceType,
    resolved: &Resolved,
    pass: Pass,
) -> FileOutcome {
    process_source(text, STDIN_NAME.to_owned(), source_type, resolved, pass)
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
        unstable: needs_rewrite.then_some(entry.unstable).flatten(),
    })
}

/// Collects the as-written diagnostics, and with `validate` guards the
/// would-be rewrite against an output that fails to re-parse or to
/// compile and against one a second pass would change.
fn diagnose_only(
    source: Source,
    resolved: &Resolved,
    validate: bool,
    notebook_index: Option<NotebookIndex>,
) -> FileOutcome {
    let file = source.source_file().clone();
    let diagnostics = resolved.pipeline.diagnose(&source);
    let unstable = if validate && has_format_change(&diagnostics) {
        match resolved.pipeline.validate(source) {
            Err(e) => return failed(ExitStatus::ConfigError, e),
            Ok(formatted) => settle_report(resolved, file.source_text(), &formatted),
        }
    } else {
        None
    };
    FileOutcome::Done {
        cached: false,
        diagnostics,
        file,
        notebook_index: notebook_index.map(Box::new),
        rewrite: Rewrite::Skipped,
        unstable,
    }
}

/// Routes a source text to the notebook or module pipeline path under
/// its diagnostic `name`.
fn process_source(
    text: String,
    name: String,
    source_type: PySourceType,
    resolved: &Resolved,
    pass: Pass,
) -> FileOutcome {
    if source_type.is_ipynb() {
        return notebook::process(text, name, resolved, pass);
    }
    match Source::build_module(text, name.as_str(), source_type) {
        Ok(source) => run_pipeline(source, resolved, pass),
        Err(e) => failed(
            ExitStatus::ParseError,
            format_args!("parse error in `{name}`: {e}"),
        ),
    }
}

/// Runs the pipeline and assembles the outcome, deferring the rewrite
/// to `rewrite`. The caller handles the diagnose-only pass, while the
/// `diagnose_as_written` flag adds the as-written diagnostics an output
/// format renders beside the rewrite. A rewritten notebook re-reads from
/// the bytes that reached disk, so a write that lost its cell boundaries
/// fails rather than being reported clean.
fn run_and_assemble(
    source: Source,
    resolved: &Resolved,
    diagnose_as_written: bool,
    notebook_index: Option<NotebookIndex>,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    let file = source.source_file().clone();
    let diagnosed = diagnose_as_written.then(|| resolved.pipeline.diagnose(&source));
    match resolved.pipeline.run(source) {
        Ok((formatted, run_diagnostics)) => {
            let rewrite = rewrite(&formatted, &file);
            if let Rewrite::Changed(kind) = &rewrite
                && formatted.is_notebook()
                && notebook::as_written(kind.written(), file.name()).is_none()
            {
                return failed(
                    ExitStatus::ConfigError,
                    format_args!("{} did not re-read as a notebook", file.name()),
                );
            }
            FileOutcome::Done {
                cached: false,
                diagnostics: diagnosed.unwrap_or(run_diagnostics),
                unstable: matches!(rewrite, Rewrite::Changed(_))
                    .then(|| settle_report(resolved, file.source_text(), &formatted))
                    .flatten(),
                file,
                notebook_index: notebook_index.map(Box::new),
                rewrite,
            }
        }
        Err(e) => failed(ExitStatus::ConfigError, e),
    }
}

/// Runs a text source through the pipeline via [`drive`], building the
/// text rewrite from the formatted output against the original. A module
/// carries no notebook index.
fn run_pipeline(source: Source, resolved: &Resolved, pass: Pass) -> FileOutcome {
    drive(source, resolved, pass, None, |formatted, file| {
        formatted
            .changed_from(file.source_text())
            .map_or(Rewrite::Unchanged, |text| Rewrite::text(text.to_owned()))
    })
}

/// The settle report over `formatted`, the run's output for `original`.
fn settle_report(
    resolved: &Resolved,
    original: &str,
    formatted: &Source,
) -> Option<Box<UnstableRewrite>> {
    UnstableRewrite::detect(&resolved.pipeline, &resolved.config, original, formatted).map(Box::new)
}

fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    failed(ExitStatus::ConfigError, format_args!("cannot walk: {err}"))
}

/// Replaces `path`'s contents with `contents` through a temporary file
/// renamed over the target, so a write that fails partway leaves the
/// original intact rather than truncated at its opening byte. `path`
/// resolves through a symlink first, leaving the link in place and
/// rewriting what it points at. Opening the target beforehand holds the
/// permission check a direct write makes, and the temporary takes the
/// target's mode, which a fresh temporary would otherwise narrow to
/// owner-only. Creating that temporary needs write permission on the
/// containing directory, which a direct write does not.
fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let target = fs_err::canonicalize(path)?;
    let permissions = fs_err::OpenOptions::new()
        .write(true)
        .open(&target)?
        .metadata()?
        .permissions();
    let mut temp = NamedTempFile::new_in(target.parent().unwrap_or(&target))?;
    temp.write_all(contents.as_bytes())?;
    temp.as_file().set_permissions(permissions)?;
    temp.persist(&target).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;
    use ruff_diagnostics::Edit;
    use tempfile::TempDir;

    use super::super::{report::status_from_outcomes, resolve::ConfigResolver};
    use super::*;
    use crate::{
        cache::RewriteKind,
        config::Config,
        pipeline::Pipeline,
        rule::RuleId,
        testing::{GroupSentinelRule, breaks_parse, never_settles, parse, range},
    };

    #[test]
    fn check_validate_fails_on_unparseable_rule_output() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(breaks_parse())]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, Pass::Diagnose { validate: true });

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn check_without_validate_builds_no_settle_report() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(never_settles(
            "widener",
        ))]));

        let outcome = run_pipeline(
            parse("x = 1\n"),
            &resolved,
            Pass::Diagnose { validate: false },
        );

        assert_matches!(outcome, FileOutcome::Done { unstable: None, .. });
    }

    #[test]
    fn check_without_validate_ignores_unparseable_rule_output() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(breaks_parse())]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, Pass::Diagnose { validate: false });

        assert_matches!(
            outcome,
            FileOutcome::Done {
                rewrite: Rewrite::Skipped,
                ..
            }
        );
    }

    #[rstest]
    fn every_pass_lands_a_rewrite_a_rule_still_edits(
        #[values(Pass::Both, Pass::Rewrite, Pass::Diagnose { validate: true })] pass: Pass,
    ) {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(never_settles(
            "widener",
        ))]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, pass);

        assert_matches!(
            &outcome,
            FileOutcome::Done {
                unstable: Some(report),
                ..
            } if report.rules == [RuleId::from("widener")]
                && report.first == "yy = 1\n"
                && report.second == "yyy = 1\n"
        );
    }

    #[rstest]
    fn no_pass_reports_a_rewrite_the_config_holds_quiet(
        #[values(Pass::Both, Pass::Rewrite, Pass::Diagnose { validate: true })] pass: Pass,
    ) {
        let mut resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(never_settles(
            "widener",
        ))]));
        resolved.config.report_unstable_output = false;

        let outcome = run_pipeline(parse("x = 1\n"), &resolved, pass);

        assert_matches!(outcome, FileOutcome::Done { unstable: None, .. });
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

    #[rstest]
    #[case::format_pass(true, true)]
    #[case::check_pass(false, false)]
    fn rehydrate_carries_the_settle_report_only_into_a_rewrite_pass(
        #[case] needs_rewrite: bool,
        #[case] carried: bool,
    ) {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::text("yy = 1\n".to_owned()),
            unstable: Some(Box::new(UnstableRewrite::sample("widener"))),
        };

        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n",
            entry,
            needs_rewrite,
        );

        assert_matches!(
            outcome,
            Some(FileOutcome::Done { unstable, .. }) if unstable.is_some() == carried
        );
    }

    #[test]
    fn rehydrate_marks_a_check_mode_outcome_skipped() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::text("y = 1\n".to_owned()),
            unstable: None,
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
            unstable: None,
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
            unstable: None,
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
            unstable: None,
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
    fn a_settled_rewrite_carries_no_report() {
        let resolved = Resolved::over(Pipeline::with_defaults(&Config::default()));

        let outcome = run_pipeline(parse("alpha = 1\nb = 22\n"), &resolved, Pass::Rewrite);

        assert_matches!(
            &outcome,
            FileOutcome::Done {
                rewrite: Rewrite::Changed(_),
                unstable: None,
                ..
            }
        );
    }

    #[test]
    fn rewrite_pass_fails_on_unparseable_rule_output() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(breaks_parse())]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, Pass::Rewrite);

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn run_pipeline_reports_unchanged_when_edits_cancel() {
        let range = range(0, 1);
        // `x-to-y` still edits the cancelled output, so a settle check
        // ahead of the `Rewrite::Changed` guard would fail this file.
        let resolved = Resolved::over(Pipeline::from_rules(vec![
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("y".to_owned(), range)]],
                id: RuleId::from("x-to-y"),
            }),
            Box::new(GroupSentinelRule {
                groups: vec![vec![Edit::range_replacement("x".to_owned(), range)]],
                id: RuleId::from("y-to-x"),
            }),
        ]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, Pass::Rewrite);

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

    #[cfg(unix)]
    #[test]
    fn write_atomic_holds_the_original_where_no_temporary_can_land() {
        use std::{fs::Permissions, os::unix::fs::PermissionsExt};

        let dir = TempDir::new().expect("a temporary directory");
        let file = dir.path().join("t.py");
        fs_err::write(&file, "x = 1\n").expect("seeds the file");
        fs_err::set_permissions(dir.path(), Permissions::from_mode(0o500)).expect("seals the dir");

        let result = write_atomic(&file, "y = 2\n");

        fs_err::set_permissions(dir.path(), Permissions::from_mode(0o700))
            .expect("reopens the dir");
        assert_matches!(result, Err(_));
        assert_eq!(fs_err::read_to_string(&file).expect("reads"), "x = 1\n");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_the_targets_mode() {
        use std::{fs::Permissions, os::unix::fs::PermissionsExt};

        let dir = TempDir::new().expect("a temporary directory");
        let file = dir.path().join("t.py");
        fs_err::write(&file, "x = 1\n").expect("seeds the file");
        fs_err::set_permissions(&file, Permissions::from_mode(0o755)).expect("sets the mode");

        write_atomic(&file, "y = 2\n").expect("writes the file");

        let mode = fs_err::metadata(&file)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        assert_eq!(fs_err::read_to_string(&file).expect("reads"), "y = 2\n");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_rewrites_through_a_symlink_leaving_the_link() {
        let dir = TempDir::new().expect("a temporary directory");
        let target = dir.path().join("real.py");
        let link = dir.path().join("link.py");
        fs_err::write(&target, "x = 1\n").expect("seeds the target");
        std::os::unix::fs::symlink(&target, &link).expect("links to the target");

        write_atomic(&link, "y = 2\n").expect("writes the file");

        assert!(link.is_symlink());
        assert_eq!(fs_err::read_to_string(&target).expect("reads"), "y = 2\n");
    }
}

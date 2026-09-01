//! Per-file processing: read, resolve config, cache-lookup, run the
//! pipeline, and classify the outcome.

use std::{
    collections::BTreeSet,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    string::FromUtf8Error,
    sync::mpsc,
};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ruff_notebook::NotebookIndex;
use ruff_python_ast::PySourceType;
use ruff_source_file::{SourceFile, SourceFileBuilder};
use rustc_hash::FxHashMap;
use tempfile::NamedTempFile;

use super::{
    FileOutcome, Pass, RunSetup, STDIN_NAME, has_format_change, notebook, resolve::Resolved,
};
use crate::{
    cache::{Anchor, CacheEntry, CacheEntryRef, NotebookCells, NotebookCellsRef, Rewrite},
    cli::exit_status::ExitStatus,
    diagnostics::fired_rules,
    rule::RuleId,
    source::Source,
    unstable::UnstableRewrite,
    walker::{self, Found},
};

/// How a run answers the settle question for one file. `Eager` walks
/// the rules over the output as it lands. The ledger variants apply
/// where a live cache lets a write-back run mark its output instead,
/// wherein the walk is skipped and `LedgerHit` proves the input bytes
/// are a prior run's own output that failed to settle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Marker {
    Eager,
    LedgerHit,
    LedgerMiss,
}

/// One walk entry's outcome paired with the block its own worker
/// rendered, `None` for an entry that yields neither.
type Landed = anyhow::Result<Option<(FileOutcome, Vec<u8>)>>;

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

/// Dispatches `source` by `pass`, collecting the as-written diagnostics
/// on a check pass and building the rewrite through `rewrite` on a
/// format pass. A notebook threads its `index`, a module passes `None`.
pub(super) fn drive(
    source: Source,
    resolved: &Resolved,
    pass: Pass,
    index: Option<NotebookIndex>,
    marker: Marker,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    if let Pass::Diagnose { validate } = pass {
        return diagnose_only(source, resolved, validate, index);
    }
    run_and_assemble(source, resolved, pass, index, marker, rewrite)
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
    let keyed = setup
        .cache_for(pass)
        .map(|c| (c, resolved.key_prefix.key_for(&bytes)));
    let hit = keyed.as_ref().and_then(|(c, k)| c.lookup(k));
    let bytes = match hit {
        Some(entry) => match rehydrate(path, bytes, entry, pass) {
            Ok(outcome) => return outcome,
            // A `check` entry cannot serve a mode needing the rewrite,
            // so the miss branch runs against the bytes it hands back.
            Err(bytes) => bytes,
        },
        None => bytes,
    };
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            return failed(
                ExitStatus::ConfigError,
                format_args!("{} is not valid UTF-8: {e}", path.display()),
            );
        }
    };
    let marker = keyed.as_ref().map_or(Marker::Eager, |(c, k)| {
        if c.owns_output(k) {
            Marker::LedgerHit
        } else {
            Marker::LedgerMiss
        }
    });
    let outcome = process_source(
        text,
        path.display().to_string(),
        source_type,
        &resolved,
        pass,
        marker,
    );
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            file,
            notebook_index,
            rewrite,
            unstable,
            ..
        },
    ) = (&keyed, &outcome)
        && worth_storing(rewrite, pass)
    {
        c.insert(
            k,
            &CacheEntryRef {
                diagnostics,
                notebook: source_type.is_ipynb().then(|| NotebookCellsRef {
                    code: file.source_text(),
                    index: notebook_index.as_deref(),
                }),
                rewrite,
                unstable: unstable.as_deref(),
            },
        );
    }
    // A write-back module run marks the bytes it is about to land, so
    // the next run holding them re-keys to this entry and a rewrite of
    // marked bytes reads as a proven settle defect.
    if let (
        Some((c, _)),
        FileOutcome::Done {
            rewrite: Rewrite::Changed(kind),
            ..
        },
    ) = (&keyed, &outcome)
        && pass.write_back()
        && !source_type.is_ipynb()
    {
        c.record_own_output(&resolved.key_prefix.key_for(kind.written().as_bytes()));
    }
    outcome
}

pub(super) fn process_paths<F>(paths: &[PathBuf], handle: F) -> Vec<FileOutcome>
where
    F: Fn(&Path, PySourceType) -> FileOutcome + Send + Sync,
{
    // Collecting the walk before the fan-out keeps the outcomes in the
    // order the walker yielded, which `par_bridge` does not, so a
    // structured report is byte-comparable between two runs.
    walker::walk(paths)
        .collect::<Vec<_>>()
        .into_par_iter()
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
    process_source(
        text,
        STDIN_NAME.to_owned(),
        source_type,
        resolved,
        pass,
        Marker::Eager,
    )
}

/// Reads stdin to a string, mapping a read failure to a config-error
/// outcome.
pub(super) fn read_stdin<R: Read>(stdin: R) -> Result<String, FileOutcome> {
    io::read_to_string(stdin)
        .map_err(|e| failed(ExitStatus::ConfigError, format_args!("reading stdin: {e}")))
}

/// Rebuilds the outcome `entry` records against the file's own
/// `bytes`, which an ordinary module hands straight to the
/// `SourceFile` rather than copying. A notebook reads the cells the
/// entry carries instead, leaving the JSON those bytes hold unparsed.
///
/// # Errors
///
/// Returns the bytes back where the entry cannot serve this mode or
/// where they are not UTF-8, so the caller runs the pipeline against
/// them without a second read.
pub(super) fn rehydrate(
    path: &Path,
    bytes: Vec<u8>,
    entry: CacheEntry,
    pass: Pass,
) -> Result<FileOutcome, Vec<u8>> {
    let (rewrite, unstable) = match (pass, entry.rewrite) {
        // A `check` renders the diagnostics alone, and builds no settle
        // report of its own to render beside them.
        (Pass::Diagnose { .. }, _) => (Rewrite::Skipped, None),
        // Any other mode needs the rewrite, which an entry a `check`
        // wrote under the shared as-written anchor never computed.
        (_, Rewrite::Skipped) => return Err(bytes),
        (_, rewrite) => (rewrite, entry.unstable),
    };
    let (source_text, notebook_index) = match entry.notebook {
        Some(NotebookCells { code, index }) => (code, index),
        None => (
            String::from_utf8(bytes).map_err(FromUtf8Error::into_bytes)?,
            None,
        ),
    };
    let file = SourceFileBuilder::new(path.display().to_string(), source_text).finish();
    Ok(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        notebook_index: notebook_index.map(Box::new),
        rewrite,
        unstable,
    })
}

/// Runs `handle` over the walk on the rayon pool and hands each file's
/// rendered block to `write` in walker order, releasing a block as soon
/// as every entry ahead of it has landed rather than once the whole
/// walk has. Rendering happens in the worker that produced the outcome,
/// so the only serial work left is the write itself. Returns the
/// outcomes in walker order, the same order [`process_paths`] returns.
pub(super) fn stream_paths<F, W>(
    paths: &[PathBuf],
    handle: F,
    mut write: W,
) -> anyhow::Result<Vec<FileOutcome>>
where
    F: Fn(&Path, PySourceType) -> anyhow::Result<(FileOutcome, Vec<u8>)> + Send + Sync,
    W: FnMut(&[u8]) -> anyhow::Result<()>,
{
    let entries: Vec<_> = walker::walk(paths).collect();
    let total = entries.len();
    let (sender, receiver) = mpsc::channel();
    let mut outcomes = Vec::with_capacity(total);
    let mut drained = Ok(());
    // The producer takes a thread of its own rather than a rayon scope,
    // because a scope holds its closure to `Send` and `write` borrows
    // the caller's stream. It fans out across the pool from there, so
    // the draining thread stays free to write what has already landed.
    std::thread::scope(|scope| {
        scope.spawn(|| {
            entries
                .into_par_iter()
                .enumerate()
                .for_each_with(sender, |sender, (slot, entry)| {
                    sender.send((slot, landed(&handle, entry))).ok();
                });
        });
        drained = drain_in_order(&receiver, total, &mut outcomes, &mut write);
    });
    drained.map(|()| outcomes)
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
    let (diagnostics, unstable) = if validate {
        match resolved.pipeline.run_as_written(source) {
            Ok((formatted, diagnostics, _)) => {
                let unstable = has_format_change(&diagnostics)
                    .then(|| settle_report(resolved, file.source_text(), &formatted))
                    .flatten();
                (diagnostics, unstable)
            }
            Err(e) => return failed(ExitStatus::ConfigError, e),
        }
    } else {
        (resolved.pipeline.diagnose(&source), None)
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

/// Writes each landed block through `write` in slot order, holding a
/// block that arrives ahead of its predecessors until they land.
fn drain_in_order<W>(
    receiver: &mpsc::Receiver<(usize, Landed)>,
    total: usize,
    outcomes: &mut Vec<FileOutcome>,
    write: &mut W,
) -> anyhow::Result<()>
where
    W: FnMut(&[u8]) -> anyhow::Result<()>,
{
    let mut held: FxHashMap<usize, Landed> = FxHashMap::default();
    let mut next = 0;
    while next < total {
        let Ok((slot, landed)) = receiver.recv() else {
            break;
        };
        held.insert(slot, landed);
        while let Some(landed) = held.remove(&next) {
            next += 1;
            if let Some((outcome, block)) = landed? {
                write(&block)?;
                outcomes.push(outcome);
            }
        }
    }
    Ok(())
}

/// Runs `handle` over one walk entry, reporting a passed-over symlink
/// on stderr and turning a walk failure into its own outcome.
fn landed<F>(handle: &F, entry: Result<Found, ignore::Error>) -> Landed
where
    F: Fn(&Path, PySourceType) -> anyhow::Result<(FileOutcome, Vec<u8>)>,
{
    match entry {
        Ok(Found::Formattable(path, source_type)) => handle(&path, source_type).map(Some),
        Ok(Found::PassedLink(path)) => {
            eprintln!("note: passed over the symlink {}", path.display());
            Ok(None)
        }
        Err(e) => Ok(Some((walk_error(e), Vec::new()))),
    }
}

/// The report a ledger probe hit mints, reading the editing set off the
/// marked input rather than walking the fresh output.
fn marked_report(
    resolved: &Resolved,
    original: &str,
    formatted: &Source,
) -> Option<Box<UnstableRewrite>> {
    UnstableRewrite::detect_marked(&resolved.pipeline, &resolved.config, original, formatted)
        .map(Box::new)
}

/// The settle report a `format` run makes over `formatted`, its output
/// for `original`, re-applying only `fired`, the rules the run's fold
/// applied.
fn narrowed_report(
    resolved: &Resolved,
    original: &str,
    formatted: &Source,
    fired: &BTreeSet<RuleId>,
) -> Option<Box<UnstableRewrite>> {
    UnstableRewrite::detect_narrowed(
        &resolved.pipeline,
        &resolved.config,
        original,
        formatted,
        fired,
    )
    .map(Box::new)
}

/// Routes a source text to the notebook or module pipeline path under
/// its diagnostic `name`.
fn process_source(
    text: String,
    name: String,
    source_type: PySourceType,
    resolved: &Resolved,
    pass: Pass,
    marker: Marker,
) -> FileOutcome {
    if source_type.is_ipynb() {
        return notebook::process(text, name, resolved, pass);
    }
    match Source::build_module(text, name.as_str(), source_type) {
        Ok(source) => run_pipeline(source, resolved, pass, marker),
        Err(e) => failed(
            ExitStatus::ParseError,
            format_args!("parse error in `{name}`: {e}"),
        ),
    }
}

/// Runs the pipeline and assembles the outcome, deferring the rewrite
/// to `rewrite`. The caller handles the diagnose-only pass, while an
/// `AsWritten` anchor takes the rewrite beside the as-written
/// diagnostics an output format renders with it. A rewritten notebook
/// re-reads from the bytes that reached disk, so a write that lost its
/// cell boundaries fails rather than being reported clean.
fn run_and_assemble(
    source: Source,
    resolved: &Resolved,
    pass: Pass,
    notebook_index: Option<NotebookIndex>,
    marker: Marker,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    let file = source.source_file().clone();
    let run = match pass.anchor() {
        Anchor::AsWritten => resolved.pipeline.run_as_written(source),
        Anchor::Rewritten => resolved
            .pipeline
            .run(source)
            .map(|(formatted, diagnostics)| {
                let fired = fired_rules(&diagnostics);
                (formatted, diagnostics, fired)
            }),
    };
    match run {
        Ok((formatted, diagnostics, fired)) => {
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
            // Under a live ledger a write-back module run skips the
            // settle walk, because its output is marked instead and a
            // probe hit on the input bytes is already the proof the
            // prior run's output failed to settle, arriving with the
            // minimal reproducing source in hand.
            let checks = match marker {
                Marker::Eager | Marker::LedgerHit => true,
                Marker::LedgerMiss => !pass.write_back() || formatted.is_notebook(),
            };
            let proven = marker == Marker::LedgerHit && pass.write_back();
            let unstable = (resolved.config.report_unstable_output
                && checks
                && (proven || matches!(rewrite, Rewrite::Changed(_))))
            .then(|| {
                if proven {
                    marked_report(resolved, file.source_text(), &formatted)
                } else {
                    narrowed_report(resolved, file.source_text(), &formatted, &fired)
                }
            })
            .flatten();
            FileOutcome::Done {
                cached: false,
                diagnostics,
                file,
                notebook_index: notebook_index.map(Box::new),
                rewrite,
                unstable,
            }
        }
        Err(e) => failed(ExitStatus::ConfigError, e),
    }
}

/// Runs a text source through the pipeline via [`drive`], building the
/// text rewrite from the formatted output against the original. A module
/// carries no notebook index.
fn run_pipeline(source: Source, resolved: &Resolved, pass: Pass, marker: Marker) -> FileOutcome {
    drive(source, resolved, pass, None, marker, |formatted, file| {
        formatted
            .changed_from(file.source_text())
            .map_or(Rewrite::Unchanged, |text| Rewrite::text(text.to_owned()))
    })
}

/// The settle report `check --validate` makes over `formatted`, the
/// run's output for `original`, re-applying every enabled rule.
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

/// True where an entry recording `rewrite` is worth writing. A
/// write-back pass commits the rewrite over the bytes its key was drawn
/// from, so the file it just wrote never reads that entry back, and a
/// `Changed` rewrite goes unwritten under such a pass. Every other
/// pairing stores.
fn worth_storing(rewrite: &Rewrite, pass: Pass) -> bool {
    !(pass.write_back() && matches!(rewrite, Rewrite::Changed(_)))
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
        cache::{Cache, RewriteKind},
        config::Config,
        pipeline::Pipeline,
        rule::RuleId,
        testing::{GroupSentinelRule, breaks_parse, never_settles, notebook_index, parse, range},
    };

    /// Seeds `dir` with one module per name and returns the walk order
    /// the driver hands them back in.
    fn seeded(dir: &TempDir, names: &[&str]) -> Vec<PathBuf> {
        for name in names {
            fs_err::write(dir.path().join(name), "x = 1\n").expect("seeds the module");
        }
        let root = vec![dir.path().to_path_buf()];
        walker::walk(&root)
            .filter_map(|entry| match entry {
                Ok(Found::Formattable(path, _)) => Some(path),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_marked_output_the_next_run_rewrites_proves_the_defect() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("a.py");
        fs_err::write(&path, "x = 1\n").expect("writes the seed");
        // Each run opens the cache afresh, as the CLI does, so the second
        // reads the ledger the first appended to.
        let run = || {
            let resolver =
                ConfigResolver::over(Resolved::over(Pipeline::from_rules(vec![Box::new(
                    never_settles("widener"),
                )])));
            let setup = RunSetup {
                cache: Some(Cache::in_store(tmp.path().join("cache"))),
                cwd: resolver.seed(&Config::default()),
                resolver,
                verbose: false,
            };
            process_path(&path, PySourceType::Python, &setup, Pass::Rewrite)
        };

        let first = run();
        assert_matches!(
            &first,
            FileOutcome::Done {
                rewrite: Rewrite::Changed(kind),
                unstable: None,
                ..
            } if kind.written() == "yy = 1\n",
            "the ledger run marks its output rather than walking it",
        );

        fs_err::write(&path, "yy = 1\n").expect("lands the rewrite");
        let second = run();
        assert_matches!(
            &second,
            FileOutcome::Done {
                unstable: Some(report),
                ..
            } if report.rules == [RuleId::from("widener")] && report.first == "yyy = 1\n",
        );
    }

    #[test]
    fn a_settled_rewrite_carries_no_report() {
        let resolved = Resolved::over(Pipeline::with_defaults(&Config::default()));

        let outcome = run_pipeline(
            parse("alpha = 1\nb = 22\n"),
            &resolved,
            Pass::Rewrite,
            Marker::Eager,
        );

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
    fn check_validate_fails_on_unparseable_rule_output() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(breaks_parse())]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(
            source,
            &resolved,
            Pass::Diagnose { validate: true },
            Marker::Eager,
        );

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
            Marker::Eager,
        );

        assert_matches!(outcome, FileOutcome::Done { unstable: None, .. });
    }

    #[test]
    fn check_without_validate_ignores_unparseable_rule_output() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(breaks_parse())]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(
            source,
            &resolved,
            Pass::Diagnose { validate: false },
            Marker::Eager,
        );

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
        #[values(Pass::Both, Pass::Preview, Pass::Rewrite, Pass::Diagnose { validate: true })]
        pass: Pass,
    ) {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(never_settles(
            "widener",
        ))]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, pass, Marker::Eager);

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
    fn no_format_pass_reports_a_rewrite_the_config_holds_quiet(
        #[values(Pass::Both, Pass::Rewrite)] pass: Pass,
    ) {
        let mut resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(never_settles(
            "widener",
        ))]));
        resolved.config.report_unstable_output = false;

        let outcome = run_pipeline(parse("x = 1\n"), &resolved, pass, Marker::Eager);

        assert_matches!(outcome, FileOutcome::Done { unstable: None, .. });
    }

    #[test]
    fn process_path_returns_config_error_on_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let resolver = ConfigResolver::new(Vec::new(), Vec::new(), Anchor::AsWritten);
        let cwd = resolver.seed(&Config::default());
        let setup = RunSetup {
            cache: None,
            cwd,
            resolver,
            verbose: false,
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
    #[case::structured_format(Pass::Both, true)]
    #[case::check(Pass::Diagnose { validate: false }, false)]
    #[case::preview(Pass::Preview, true)]
    #[case::rewrite(Pass::Rewrite, true)]
    fn rehydrate_carries_the_settle_report_only_into_a_rewriting_pass(
        #[case] pass: Pass,
        #[case] carried: bool,
    ) {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::text("yy = 1\n".to_owned()),
            unstable: Some(Box::new(UnstableRewrite::sample("widener"))),
        };

        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n".to_vec(), entry, pass);

        assert_matches!(
            outcome,
            Ok(FileOutcome::Done { unstable, .. }) if unstable.is_some() == carried
        );
    }

    #[test]
    fn rehydrate_hands_the_bytes_back_for_a_skipped_entry() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::Skipped,
            unstable: None,
        };
        assert_matches!(
            rehydrate(Path::new("a.py"), b"x = 1\n".to_vec(), entry, Pass::Both),
            Err(bytes) if bytes == b"x = 1\n"
        );
    }

    #[test]
    fn rehydrate_marks_a_check_mode_outcome_skipped() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::text("y = 1\n".to_owned()),
            unstable: None,
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            b"x = 1\n".to_vec(),
            entry,
            Pass::Diagnose { validate: false },
        );
        assert_matches!(
            outcome,
            Ok(FileOutcome::Done {
                rewrite: Rewrite::Skipped,
                ..
            })
        );
    }

    #[test]
    fn rehydrate_reads_a_notebook_entry_off_its_stored_cells() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: Some(NotebookCells {
                code: "x = 1\n".to_owned(),
                index: Some(notebook_index(&["x = 1\n"])),
            }),
            rewrite: Rewrite::Skipped,
            unstable: None,
        };

        // The bytes are no notebook at all, so an outcome carrying the
        // cells proves the hit never read them back as JSON.
        let outcome = rehydrate(
            Path::new("nb.ipynb"),
            b"not a notebook".to_vec(),
            entry,
            Pass::Diagnose { validate: false },
        );

        assert_matches!(
            outcome,
            Ok(FileOutcome::Done { file, notebook_index: Some(index), .. })
                if file.source_text() == "x = 1\n" && *index == notebook_index(&["x = 1\n"])
        );
    }

    #[test]
    fn rehydrate_serves_a_changed_rewrite_to_a_format_mode() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::text("y = 1\n".to_owned()),
            unstable: None,
        };
        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n".to_vec(), entry, Pass::Both);
        assert_matches!(
            outcome,
            Ok(FileOutcome::Done { rewrite: Rewrite::Changed(RewriteKind::Text(text)), .. })
                if text == "y = 1\n"
        );
    }

    #[test]
    fn rehydrate_serves_a_passed_over_rewrite_to_a_text_format() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::PassedOver,
            unstable: None,
        };
        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n".to_vec(), entry, Pass::Rewrite);
        assert_matches!(
            outcome,
            Ok(FileOutcome::Done {
                rewrite: Rewrite::PassedOver,
                ..
            })
        );
    }

    #[test]
    fn rehydrate_serves_an_unchanged_rewrite_as_no_edit() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            notebook: None,
            rewrite: Rewrite::Unchanged,
            unstable: None,
        };
        let outcome = rehydrate(Path::new("a.py"), b"x = 1\n".to_vec(), entry, Pass::Both);
        assert_matches!(
            outcome,
            Ok(FileOutcome::Done {
                rewrite: Rewrite::Unchanged,
                ..
            })
        );
    }

    #[test]
    fn rewrite_pass_fails_on_unparseable_rule_output() {
        let resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(breaks_parse())]));
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &resolved, Pass::Rewrite, Marker::Eager);

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

        let outcome = run_pipeline(source, &resolved, Pass::Rewrite, Marker::Eager);

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

    #[cfg(unix)]
    #[test]
    fn stream_paths_passes_over_a_symlink_without_a_block() {
        let dir = TempDir::new().expect("a temporary directory");
        seeded(&dir, &["a.py"]);
        std::os::unix::fs::symlink(dir.path().join("a.py"), dir.path().join("link.py"))
            .expect("links to the module");
        let mut blocks = 0;

        let outcomes = stream_paths(
            &[dir.path().to_path_buf()],
            |_, _| Ok((FileOutcome::Failed(ExitStatus::Clean), Vec::new())),
            |_| {
                blocks += 1;
                Ok(())
            },
        )
        .expect("the walk streams");

        assert_eq!(blocks, 1);
        assert_eq!(outcomes.len(), 1);
    }

    #[test]
    fn stream_paths_returns_the_writers_failure() {
        let dir = TempDir::new().expect("a temporary directory");
        seeded(&dir, &["a.py"]);

        let result = stream_paths(
            &[dir.path().to_path_buf()],
            |_, _| Ok((FileOutcome::Failed(ExitStatus::Clean), b"block".to_vec())),
            |_| Err(anyhow::anyhow!("the writer declined")),
        );

        assert_eq!(
            result.expect_err("the writer failure surfaces").to_string(),
            "the writer declined",
        );
    }

    #[test]
    fn stream_paths_writes_each_block_in_walker_order() {
        let dir = TempDir::new().expect("a temporary directory");
        let order = seeded(&dir, &["a.py", "b.py", "c.py", "d.py"]);
        let first = order.first().expect("the walk found a module").clone();
        let mut written = Vec::new();

        let outcomes = stream_paths(
            &[dir.path().to_path_buf()],
            |path, _| {
                // Holding the block the walk yields first behind every
                // other one forces the driver to reorder rather than
                // write in completion order.
                if path == first {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Ok((
                    FileOutcome::Failed(ExitStatus::Clean),
                    path.display().to_string().into_bytes(),
                ))
            },
            |block| {
                written.push(String::from_utf8(block.to_vec()).expect("the block is UTF-8"));
                Ok(())
            },
        )
        .expect("the walk streams");

        let expected: Vec<String> = order.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(written, expected);
        assert_eq!(outcomes.len(), expected.len());
    }

    #[test]
    fn validate_reports_a_rewrite_despite_the_config_key() {
        let mut resolved = Resolved::over(Pipeline::from_rules(vec![Box::new(never_settles(
            "widener",
        ))]));
        resolved.config.report_unstable_output = false;

        let outcome = run_pipeline(
            parse("x = 1\n"),
            &resolved,
            Pass::Diagnose { validate: true },
            Marker::Eager,
        );

        assert_matches!(
            outcome,
            FileOutcome::Done {
                unstable: Some(_),
                ..
            }
        );
    }

    #[test]
    fn walk_error_returns_failed_with_config_error() {
        let outcome = walk_error("synthetic walk failure");
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[cfg(unix)]
    #[rstest]
    #[case::changed_under_a_commit(Rewrite::text("y = 1\n".to_owned()), Pass::Rewrite, false)]
    #[case::changed_under_a_structured_commit(Rewrite::text("y = 1\n".to_owned()), Pass::Both, false)]
    #[case::changed_under_a_preview(Rewrite::text("y = 1\n".to_owned()), Pass::Preview, true)]
    #[case::unchanged_under_a_commit(Rewrite::Unchanged, Pass::Rewrite, true)]
    #[case::passed_over_under_a_commit(Rewrite::PassedOver, Pass::Rewrite, true)]
    #[case::skipped_under_a_check(Rewrite::Skipped, Pass::Diagnose { validate: false }, true)]
    fn worth_storing_drops_only_a_rewrite_a_committing_pass_lands(
        #[case] rewrite: Rewrite,
        #[case] pass: Pass,
        #[case] stores: bool,
    ) {
        assert_eq!(worth_storing(&rewrite, pass), stores);
    }

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

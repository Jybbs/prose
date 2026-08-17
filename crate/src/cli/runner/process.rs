//! Per-file processing: read, resolve config, cache-lookup, run the
//! pipeline, and classify the outcome.

use std::{
    collections::HashMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    string::FromUtf8Error,
    sync::mpsc,
};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ruff_notebook::NotebookIndex;
use ruff_python_ast::PySourceType;
use ruff_source_file::{SourceFile, SourceFileBuilder};
use tempfile::NamedTempFile;

use super::{FileOutcome, Pass, RunSetup, notebook};
use crate::{
    cache::{Anchor, CacheEntry, CacheEntryRef, Rewrite},
    cli::exit_status::ExitStatus,
    pipeline::Pipeline,
    source::Source,
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
    pipeline: &Pipeline,
    pass: Pass,
    index: Option<NotebookIndex>,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    if let Pass::Diagnose { validate } = pass {
        return diagnose_only(source, pipeline, validate, index);
    }
    run_and_assemble(source, pipeline, pass.anchor(), index, rewrite)
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
        Some(entry) => match rehydrate(path, source_type, bytes, entry, pass) {
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
    let outcome = process_source(
        text,
        path.display().to_string(),
        source_type,
        &resolved.pipeline,
        pass,
    );
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            rewrite,
            ..
        },
    ) = (&keyed, &outcome)
        && worth_storing(rewrite, pass)
    {
        c.insert(
            k,
            &CacheEntryRef {
                diagnostics,
                rewrite,
            },
        );
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

/// One walk entry's outcome paired with the block its own worker
/// rendered, `None` for an entry that yields neither.
type Landed = anyhow::Result<Option<(FileOutcome, Vec<u8>)>>;

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
    let mut held: HashMap<usize, Landed> = HashMap::new();
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

pub(super) fn process_stdin(
    text: String,
    source_type: PySourceType,
    pipeline: &Pipeline,
    pass: Pass,
) -> FileOutcome {
    process_source(text, "<stdin>".to_owned(), source_type, pipeline, pass)
}

/// Reads stdin to a string, mapping a read failure to a config-error
/// outcome.
pub(super) fn read_stdin<R: Read>(stdin: R) -> Result<String, FileOutcome> {
    io::read_to_string(stdin)
        .map_err(|e| failed(ExitStatus::ConfigError, format_args!("reading stdin: {e}")))
}

/// Rebuilds the outcome `entry` records against the file's own
/// `bytes`, which an ordinary module hands straight to the
/// `SourceFile` rather than copying.
///
/// # Errors
///
/// Returns the bytes back where the entry cannot serve this mode,
/// where they are not UTF-8, or where a notebook fails to re-read, so
/// the caller runs the pipeline against them without a second read.
pub(super) fn rehydrate(
    path: &Path,
    source_type: PySourceType,
    bytes: Vec<u8>,
    entry: CacheEntry,
    pass: Pass,
) -> Result<FileOutcome, Vec<u8>> {
    let rewrite = match (pass, entry.rewrite) {
        // A `check` renders the diagnostics alone.
        (Pass::Diagnose { .. }, _) => Rewrite::Skipped,
        // Any other mode needs the rewrite, which an entry a `check`
        // wrote under the shared as-written anchor never computed.
        (_, Rewrite::Skipped) => return Err(bytes),
        (_, rewrite) => rewrite,
    };
    let text = String::from_utf8(bytes).map_err(FromUtf8Error::into_bytes)?;
    let (source_text, notebook_index) = if source_type.is_ipynb() {
        match notebook::rehydrated(&text) {
            Some((code, index)) => (code, Some(index)),
            None => return Err(text.into_bytes()),
        }
    } else {
        (text, None)
    };
    let file = SourceFileBuilder::new(path.display().to_string(), source_text).finish();
    Ok(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        notebook_index: notebook_index.map(Box::new),
        rewrite,
    })
}

/// True where an entry recording `rewrite` is worth writing. A
/// write-back pass commits the rewrite over the bytes its key was drawn
/// from, so the file it just wrote never reads that entry back, and a
/// `Changed` rewrite goes unwritten under such a pass. Every other
/// pairing stores.
pub(super) fn worth_storing(rewrite: &Rewrite, pass: Pass) -> bool {
    !(pass.write_back() && matches!(rewrite, Rewrite::Changed(_)))
}

/// Collects the as-written diagnostics, and with `validate` guards the
/// would-be rewrite against an output that fails to re-parse or to
/// compile.
fn diagnose_only(
    source: Source,
    pipeline: &Pipeline,
    validate: bool,
    notebook_index: Option<NotebookIndex>,
) -> FileOutcome {
    let file = source.source_file().clone();
    let diagnostics = if validate {
        match pipeline.validated(source) {
            Ok(diagnostics) => diagnostics,
            Err(e) => return failed(ExitStatus::ConfigError, e),
        }
    } else {
        pipeline.diagnose(&source)
    };
    FileOutcome::Done {
        cached: false,
        diagnostics,
        file,
        notebook_index: notebook_index.map(Box::new),
        rewrite: Rewrite::Skipped,
    }
}

/// Routes a source text to the notebook or module pipeline path under
/// its diagnostic `name`.
fn process_source(
    text: String,
    name: String,
    source_type: PySourceType,
    pipeline: &Pipeline,
    pass: Pass,
) -> FileOutcome {
    if source_type.is_ipynb() {
        return notebook::process(text, name, pipeline, pass);
    }
    match Source::build_module(text, name.as_str(), source_type) {
        Ok(source) => run_pipeline(source, pipeline, pass),
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
    pipeline: &Pipeline,
    anchor: Anchor,
    notebook_index: Option<NotebookIndex>,
    rewrite: impl FnOnce(&Source, &SourceFile) -> Rewrite,
) -> FileOutcome {
    let file = source.source_file().clone();
    let run = match anchor {
        Anchor::AsWritten => pipeline.run_as_written(source),
        Anchor::Rewritten => pipeline.run(source),
    };
    match run {
        Ok((formatted, diagnostics)) => {
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
                diagnostics,
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
fn run_pipeline(source: Source, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    drive(source, pipeline, pass, None, |formatted, file| {
        formatted
            .changed_from(file.source_text())
            .map_or(Rewrite::Unchanged, |text| Rewrite::text(text.to_owned()))
    })
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
        rule::RuleId,
        testing::{GroupSentinelRule, breaks_parse, never_settles, parse, range},
    };

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

    #[rstest]
    fn every_pass_lands_a_rewrite_a_rule_still_edits(
        #[values(Pass::Both, Pass::Preview, Pass::Rewrite, Pass::Diagnose { validate: true })]
        pass: Pass,
    ) {
        let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &pipeline, pass);

        assert_matches!(outcome, FileOutcome::Done { .. });
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

    #[test]
    fn rehydrate_hands_the_bytes_back_for_a_skipped_entry() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::Skipped,
        };
        assert_matches!(
            rehydrate(
                Path::new("a.py"),
                PySourceType::Python,
                b"x = 1\n".to_vec(),
                entry,
                Pass::Both
            ),
            Err(bytes) if bytes == b"x = 1\n"
        );
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
    fn rehydrate_serves_a_changed_rewrite_to_a_format_mode() {
        let entry = CacheEntry {
            diagnostics: Vec::new(),
            rewrite: Rewrite::text("y = 1\n".to_owned()),
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n".to_vec(),
            entry,
            Pass::Both,
        );
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
            rewrite: Rewrite::PassedOver,
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n".to_vec(),
            entry,
            Pass::Rewrite,
        );
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
            rewrite: Rewrite::Unchanged,
        };
        let outcome = rehydrate(
            Path::new("a.py"),
            PySourceType::Python,
            b"x = 1\n".to_vec(),
            entry,
            Pass::Both,
        );
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
        let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
        let source = parse("x = 1\n");

        let outcome = run_pipeline(source, &pipeline, Pass::Rewrite);

        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    #[test]
    fn run_pipeline_reports_unchanged_when_edits_cancel() {
        let range = range(0, 1);
        // `x-to-y` still edits the cancelled output, so a settle check
        // ahead of the `Rewrite::Changed` guard would fail this file.
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

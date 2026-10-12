//! The parallel walk over the input paths, its outcomes returned in
//! walker order and its rendered blocks drained to the stream in that
//! order as each lands.

use std::{
    convert,
    path::{Path, PathBuf},
    sync::mpsc,
};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ruff_python_ast::PySourceType;
use rustc_hash::FxHashMap;

use super::{FileOutcome, Pass, process::failed};
use crate::{
    cli::exit_status::ExitStatus,
    walker::{self, Found},
};

/// One walk entry's outcome paired with the block its own worker
/// rendered, `None` for an entry that yields neither.
type Landed = anyhow::Result<Option<(FileOutcome, Vec<u8>)>>;

/// Runs `handle` over each formattable file the walk of `paths` yields,
/// on the rayon pool, returning the outcomes in walker order. A rejected
/// path fails with an error naming the subcommand of `pass`.
pub(super) fn process_paths<F>(paths: &[PathBuf], pass: Pass, handle: F) -> Vec<FileOutcome>
where
    F: Fn(&Path, PySourceType) -> FileOutcome + Send + Sync,
{
    // Collecting the walk before the fan-out keeps the outcomes in the
    // order the walker yielded.
    walker::walk(paths)
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|entry| handled(entry, pass, &handle, convert::identity))
        .collect()
}

/// Runs `handle` over the walk on the rayon pool and hands each file's
/// rendered block to `write` in walker order, releasing a block as soon
/// as every entry ahead of it has landed. Rendering happens in the
/// worker that produced the outcome. Returns the outcomes in walker
/// order, the same order [`process_paths`] returns.
pub(super) fn stream_paths<F, W>(
    paths: &[PathBuf],
    pass: Pass,
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
    // The producer takes a thread of its own, because a rayon scope
    // holds its closure to `Send` and `write` borrows the caller's
    // stream. It fans out across the pool from there while this thread
    // drains.
    std::thread::scope(|scope| {
        scope.spawn(|| {
            entries
                .into_par_iter()
                .enumerate()
                .for_each_with(sender, |sender, (slot, entry)| {
                    sender.send((slot, landed(&handle, entry, pass))).ok();
                });
        });
        drained = drain_in_order(&receiver, total, &mut outcomes, &mut write);
    });
    drained.map(|()| outcomes)
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

/// Runs `handle` over the file one walk entry names. A rejected path or
/// a walk error fails without reaching `handle`, its outcome wrapped
/// through `lift`, and a passed-over symlink returns `None` after a note
/// on stderr.
fn handled<T>(
    entry: Result<Found, ignore::Error>,
    pass: Pass,
    handle: impl FnOnce(&Path, PySourceType) -> T,
    lift: impl FnOnce(FileOutcome) -> T,
) -> Option<T> {
    match entry {
        Ok(Found::Formattable(path, source_type)) => Some(handle(&path, source_type)),
        Ok(Found::PassedLink(path)) => {
            passed_link(&path);
            None
        }
        Ok(Found::Rejected(path)) => Some(lift(rejected(&path, pass))),
        Err(e) => Some(lift(walk_error(e))),
    }
}

/// Runs `handle` over one walk entry, landing a failure the walk reaches
/// without `handle` beside an empty block.
fn landed<F>(handle: &F, entry: Result<Found, ignore::Error>, pass: Pass) -> Landed
where
    F: Fn(&Path, PySourceType) -> anyhow::Result<(FileOutcome, Vec<u8>)>,
{
    handled(entry, pass, handle, |outcome| Ok((outcome, Vec::new()))).transpose()
}

/// Reports a symlink the walk passed over.
fn passed_link(path: &Path) {
    eprintln!("note: passed over the symlink {}", path.display());
}

/// Fails a named path the walk rejected, with an error naming the
/// subcommand of `pass`.
fn rejected(path: &Path, pass: Pass) -> FileOutcome {
    failed(
        ExitStatus::ConfigError,
        format_args!(
            "cannot {} `{}`, which is not a `.py`, `.pyi`, `.pyw`, or `.ipynb` file",
            pass.subcommand(),
            path.display(),
        ),
    )
}

fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    failed(ExitStatus::ConfigError, format_args!("cannot walk: {err}"))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use tempfile::TempDir;

    use super::*;
    use crate::testing::formattable;

    const CHECK: Pass = Pass::Diagnose { validate: false };

    /// Seeds `dir` with a script and a module, returning both with the
    /// script named first.
    fn script_and_module(dir: &TempDir) -> [PathBuf; 2] {
        let named = ["script", "a.py"].map(|name| dir.path().join(name));
        for path in &named {
            fs_err::write(path, "x = 1\n").expect("seeds the file");
        }
        named
    }

    /// Seeds `dir` with one module per name and returns the walk order
    /// the driver hands them back in.
    fn seeded(dir: &TempDir, names: &[&str]) -> Vec<PathBuf> {
        for name in names {
            fs_err::write(dir.path().join(name), "x = 1\n").expect("seeds the module");
        }
        formattable(&[dir.path().to_path_buf()])
    }

    #[test]
    fn process_paths_fails_a_rejected_path_and_runs_the_rest() {
        let dir = TempDir::new().expect("a temporary directory");

        let outcomes = process_paths(&script_and_module(&dir), CHECK, |_, _| {
            FileOutcome::Failed(ExitStatus::Clean)
        });

        assert_matches!(
            outcomes.as_slice(),
            [
                FileOutcome::Failed(ExitStatus::ConfigError),
                FileOutcome::Failed(ExitStatus::Clean),
            ]
        );
    }

    #[test]
    fn stream_paths_fails_a_rejected_path_without_a_block() {
        let dir = TempDir::new().expect("a temporary directory");
        let mut written = Vec::new();

        let outcomes = stream_paths(
            &script_and_module(&dir),
            CHECK,
            |_, _| Ok((FileOutcome::Failed(ExitStatus::Clean), b"block".to_vec())),
            |block| {
                written.push(block.to_vec());
                Ok(())
            },
        )
        .expect("the walk streams");

        assert_eq!(written, [Vec::new(), b"block".to_vec()]);
        assert_matches!(
            outcomes.as_slice(),
            [
                FileOutcome::Failed(ExitStatus::ConfigError),
                FileOutcome::Failed(ExitStatus::Clean),
            ]
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
            CHECK,
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
            CHECK,
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
            CHECK,
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
    fn walk_error_returns_failed_with_config_error() {
        let outcome = walk_error("synthetic walk failure");
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }
}

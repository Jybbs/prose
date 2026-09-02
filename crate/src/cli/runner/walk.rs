//! The parallel walk over the input paths, its outcomes returned in
//! walker order and its rendered blocks drained to the stream in that
//! order as each lands.

use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ruff_python_ast::PySourceType;
use rustc_hash::FxHashMap;

use super::FileOutcome;
use super::process::failed;
use crate::{
    cli::exit_status::ExitStatus,
    walker::{self, Found},
};

/// One walk entry's outcome paired with the block its own worker
/// rendered, `None` for an entry that yields neither.
type Landed = anyhow::Result<Option<(FileOutcome, Vec<u8>)>>;

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
                passed_link(&path);
                None
            }
            Err(e) => Some(walk_error(e)),
        })
        .collect()
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
            passed_link(&path);
            Ok(None)
        }
        Err(e) => Ok(Some((walk_error(e), Vec::new()))),
    }
}

/// Reports a symlink the walk passed over.
fn passed_link(path: &Path) {
    eprintln!("note: passed over the symlink {}", path.display());
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
    fn walk_error_returns_failed_with_config_error() {
        let outcome = walk_error("synthetic walk failure");
        assert_matches!(outcome, FileOutcome::Failed(ExitStatus::ConfigError));
    }

    /// Seeds `dir` with one module per name and returns the walk order
    /// the driver hands them back in.
    fn seeded(dir: &TempDir, names: &[&str]) -> Vec<PathBuf> {
        for name in names {
            fs_err::write(dir.path().join(name), "x = 1\n").expect("seeds the module");
        }
        formattable(&[dir.path().to_path_buf()])
    }
}

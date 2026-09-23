//! The scratch stage one sweep works in, meaning the original copy of the
//! corpus, each formatted copy, the overlays formatted under one rule, each
//! removed once its run ends, and the home, records, and temporary
//! directories the runs write to.

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Mutex,
};

use ignore::WalkBuilder;
use tempfile::TempDir;

use crate::corpus::VENDORED;

/// The probe the sweep writes into the stage and runs each module through.
const PROBE: &str = include_str!("probe.py");

/// The scratch directory one sweep works in.
pub(crate) struct Stage {
    /// The directory holding everything below, removed when the stage
    /// drops unless [`Stage::keep`] released it.
    held: Mutex<Option<TempDir>>,
    /// A scratch `HOME` for the runs.
    pub(crate) home: PathBuf,
    /// The unformatted copy every other copy and every comparison reads.
    pub(crate) original: PathBuf,
    /// Where each run writes what it left behind.
    pub(crate) records: PathBuf,
    /// The scratch directory holding everything below.
    pub(crate) root: PathBuf,
    /// A scratch `TMPDIR`, and the working directory of every run.
    pub(crate) tmp: PathBuf,
}

impl Stage {
    /// Builds the stage, copying the corpus once as the original tree and
    /// writing the probe beside it.
    pub(crate) fn new(corpus: &Path) -> Self {
        let held = tempfile::Builder::new()
            .prefix("prose-imports.")
            .tempdir()
            .expect("create the stage directory");
        let root = held.path().to_path_buf();
        let stage = Self {
            held: Mutex::new(Some(held)),
            home: root.join("home"),
            original: root.join("original"),
            records: root.join("records"),
            root: root.clone(),
            tmp: root.join("tmp"),
        };
        for directory in [&stage.home, &stage.records, &stage.tmp] {
            fs_err::create_dir_all(directory).expect("create a stage directory");
        }
        fs_err::write(stage.probe(), PROBE).expect("write the probe");
        copy_tree(corpus, &stage.original);
        stage
    }

    /// Copies the original into the stage under `name`.
    pub(crate) fn copy(&self, name: &str) -> PathBuf {
        let tree = self.root.join(name);
        copy_tree(&self.original, &tree);
        tree
    }

    /// Builds a tree holding the original of the top-level module or package
    /// carrying each of `files`, ready to be formatted under one rule ahead
    /// of the original tree, and removed when the returned directory drops.
    /// A file under [`VENDORED`] takes its top-level package from the
    /// component below that directory.
    pub(crate) fn overlay(
        &self,
        files: &[String],
        label: &str,
        module: &str,
        slug: &str,
    ) -> TempDir {
        let parent = self
            .root
            .join("alone")
            .join(label)
            .join(module.replace('/', "+"));
        fs_err::create_dir_all(&parent).expect("create an overlay parent");
        let held = tempfile::Builder::new()
            .prefix(&format!("{slug}."))
            .tempdir_in(&parent)
            .expect("create an overlay");
        let tree = held.path();
        for top in files
            .iter()
            .map(|file| {
                let path = Path::new(file);
                let depth = if path.starts_with(VENDORED) { 2 } else { 1 };
                path.iter().take(depth).collect::<PathBuf>()
            })
            .collect::<BTreeSet<_>>()
        {
            let source = self.original.join(&top);
            let target = tree.join(&top);
            if source.is_dir() {
                copy_tree(&source, &target);
            } else if source.is_file() {
                if let Some(parent) = target.parent() {
                    fs_err::create_dir_all(parent).expect("create an overlay parent");
                }
                fs_err::copy(&source, target).expect("copy an overlay module");
            }
        }
        held
    }

    /// Releases the stage from removal, so a failed run leaves its tree
    /// on disk to inspect.
    pub(crate) fn keep(&self) {
        if let Some(held) = self.held.lock().expect("the stage lock").take() {
            let _ = held.keep();
        }
    }

    /// The probe every run goes through.
    pub(crate) fn probe(&self) -> PathBuf {
        self.root.join("probe.py")
    }
}

/// Copies every file under `from` into `to`, leaving the bytecode caches
/// behind.
fn copy_tree(from: &Path, to: &Path) {
    for entry in WalkBuilder::new(from)
        .filter_entry(|entry| entry.file_name() != OsStr::new("__pycache__"))
        .standard_filters(false)
        .build()
        .flatten()
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(from)
            .unwrap_or_else(|_| unreachable!("invariant: the walk is rooted at `from`"));
        let target = to.join(relative);
        if path.is_dir() {
            fs_err::create_dir_all(&target).expect("create a copied directory");
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                fs_err::create_dir_all(parent).expect("create a copied parent");
            }
            fs_err::copy(path, &target).expect("copy a corpus file");
        }
    }
}

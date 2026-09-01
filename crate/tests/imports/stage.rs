//! The scratch stage one sweep works in, meaning the original copy of the
//! corpus, each formatted copy, the overlays formatted under one rule, and
//! the home, records, and temporary directories the runs write to.

use std::{
    collections::BTreeSet,
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process,
};

use ignore::WalkBuilder;

/// The probe the sweep writes into the stage and runs each module through.
const PROBE: &str = include_str!("probe.py");

/// The scratch directory one sweep works in.
pub(crate) struct Stage {
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
        let root = env::temp_dir().join(format!("prose-imports.{}", process::id()));
        let _ = fs_err::remove_dir_all(&root);
        let stage = Self {
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
    /// of the original tree.
    pub(crate) fn overlay(
        &self,
        files: &[String],
        label: &str,
        module: &str,
        slug: &str,
    ) -> PathBuf {
        let tree = self
            .root
            .join("alone")
            .join(label)
            .join(module.replace('/', "+"))
            .join(slug);
        fs_err::create_dir_all(&tree).expect("create an overlay");
        for top in files
            .iter()
            .map(|file| file.split_once('/').map_or(file.as_str(), |(top, _)| top))
            .collect::<BTreeSet<_>>()
        {
            let source = self.original.join(top);
            if source.is_dir() {
                copy_tree(&source, &tree.join(top));
            } else if source.is_file() {
                fs_err::copy(&source, tree.join(top)).expect("copy an overlay module");
            }
        }
        tree
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

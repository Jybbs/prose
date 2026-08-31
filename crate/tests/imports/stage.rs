//! The scratch stage one sweep works in, meaning the original copy of the
//! corpus, each formatted copy, the overlays formatted under one rule, and
//! the home, records, and temporary directories the runs write to.

use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
    process,
};

use ignore::WalkBuilder;

/// The probe the sweep writes into the stage and runs each module through.
const PROBE: &str = include_str!("probe.py");

/// The scratch directory one sweep works in.
pub(crate) struct Stage {
    /// The corpus every copy is taken from.
    corpus: PathBuf,
    /// A scratch `HOME` for the runs.
    pub(crate) home: PathBuf,
    /// The unformatted copy every comparison reads.
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
            corpus: corpus.to_path_buf(),
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

    /// Copies the corpus into the stage under `name`.
    pub(crate) fn copy(&self, name: &str) -> PathBuf {
        let tree = self.root.join(name);
        copy_tree(&self.corpus, &tree);
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
            .filter_map(|file| file.split('/').next())
            .collect::<BTreeSet<_>>()
        {
            let source = self.original.join(top);
            if source.is_dir() {
                copy_tree(&source, &tree.join(top));
            } else if source.is_file() {
                let _ = fs_err::copy(&source, tree.join(top));
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
        .standard_filters(false)
        .build()
        .flatten()
    {
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(from) else {
            continue;
        };
        if relative
            .components()
            .any(|part| part.as_os_str() == "__pycache__")
        {
            continue;
        }
        let target = to.join(relative);
        if path.is_dir() {
            fs_err::create_dir_all(&target).expect("create a copied directory");
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                fs_err::create_dir_all(parent).expect("create a copied parent");
            }
            let _ = fs_err::copy(path, &target);
        }
    }
}

//! Which modules of a corpus a sweep runs, meaning the interpreter owning
//! the corpus and the versions it reports, the entry points a run leaves out,
//! the modules a format run read, and what the run's header names about the
//! corpus.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

use itertools::Itertools;
use ruff_python_ast::PythonVersion;

use crate::{
    common::{python_files, setting},
    records::Corpus,
};

/// The module names a walk leaves out wherever they sit, since running
/// either one launches a program instead of binding a namespace.
/// `python-config.py` sits under a directory named for the platform, so
/// matching on the name rather than the path works on every machine.
const ENTRY_NAMES: &[&str] = &["__main__", "python-config"];

/// The modules a walk leaves out by their full path, so a module of the
/// same name deeper in the tree stays in.
const ENTRY_POINTS: &[&str] = &["antigravity", "idlelib/idle", "webbrowser"];

/// The directories a walk leaves out wholesale.
const ENTRY_TREES: &[&str] = &["idle_test", "test", "tests", "turtledemo"];

/// The interpreter a sweep runs absent [`PYTHON_VAR`].
const PYTHON: &str = "python3";

/// The environment variable naming the interpreter whose standard library
/// the sweep runs.
pub(crate) const PYTHON_VAR: &str = "PROSE_IMPORTS_PYTHON";

/// The directory beside the standard library that the distributions it
/// carries are installed into, which an import searches as a root of its own.
pub(crate) const VENDORED: &str = "site-packages";

/// The modules a sweep runs, which is every importable module the format
/// run read outside the entry points, sorted.
pub(crate) fn candidates(read: &BTreeSet<String>) -> Vec<String> {
    read.iter()
        .filter(|relative| importable(relative) && !excluded(relative))
        .cloned()
        .collect()
}

/// Reports whether a module is an entry point rather than a library module.
/// The name is matched with its extension removed, so `idlelib/idle.pyw` is
/// excluded along with `idlelib/idle.py`. Every path the walk yields carries an
/// extension, and both callers pass one.
pub(crate) fn excluded(relative: &str) -> bool {
    let stem = relative.rsplit_once('.').map_or(relative, |(stem, _)| stem);
    let (directories, last) = stem.rsplit_once('/').unwrap_or(("", stem));
    ENTRY_NAMES.contains(&last)
        || ENTRY_POINTS.contains(&stem)
        || directories
            .split('/')
            .any(|part| ENTRY_TREES.contains(&part))
}

/// What the run's header names about the corpus at `root`, being how many
/// files the walk reads beside the distributions installed next to the
/// standard library.
pub(crate) fn identity(root: &Path) -> Corpus {
    Corpus {
        files: python_files(root).count(),
        vendored: vendored(root),
    }
}

/// Reports whether a file names a module an import can bind. A `.pyw`
/// script and a `.pyi` stub both carry Python the formatter rewrites without
/// naming an importable module, so the walk formats them and this run leaves
/// them out.
pub(crate) fn importable(relative: &str) -> bool {
    relative.ends_with(".py")
}

/// The interpreter every module runs under, resolved to the executable it
/// runs as, with [`PYTHON_VAR`] naming it and [`PYTHON`] standing in where it
/// is unset. A version manager's shim reads its configuration from the
/// environment, which a run clears, so a run launches the executable the
/// shim resolves to instead.
pub(crate) fn interpreter() -> String {
    asked(
        &setting(PYTHON_VAR).unwrap_or_else(|| PYTHON.to_owned()),
        "import sys; print(sys.executable)",
    )
}

/// Asks an interpreter which standard library it owns.
pub(crate) fn standard_library(python: &str) -> PathBuf {
    PathBuf::from(asked(
        python,
        "import sysconfig; print(sysconfig.get_paths()['stdlib'])",
    ))
    .canonicalize()
    .expect("the interpreter names a standard library")
}

/// The `major.minor` version a full `version` names, which a format run
/// takes as its `target-version`.
pub(crate) fn target(version: &str) -> PythonVersion {
    let (major, minor) = version
        .split('.')
        .next_tuple()
        .expect("the interpreter reports a `major.minor.patch` version");
    PythonVersion::try_from((major, minor))
        .expect("the interpreter reports a `major.minor` version")
}

/// The version the interpreter at `python` reports.
pub(crate) fn version(python: &str) -> String {
    asked(python, "import sys; print(sys.version.split()[0])")
}

/// What `python` prints for `code`, trimmed. Panics if the interpreter does not
/// run or the snippet fails.
fn asked(python: &str, code: &str) -> String {
    let ran = Command::new(python)
        .args(["-I", "-c", code])
        .output()
        .unwrap_or_else(|error| panic!("{python} does not run: {error}"));
    assert!(
        ran.status.success(),
        "{python} does not run: {}",
        String::from_utf8_lossy(&ran.stderr).trim()
    );
    String::from_utf8_lossy(&ran.stdout).trim().to_owned()
}

/// Every distribution installed beside the standard library at `root`, read
/// off the `dist-info` directory each one leaves, sorted. A tree carrying no
/// [`VENDORED`] directory names none.
fn vendored(root: &Path) -> Vec<String> {
    let Ok(entries) = fs_err::read_dir(root.join(VENDORED)) else {
        return Vec::new();
    };
    let mut found: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".dist-info").map(str::to_owned)
        })
        .collect();
    found.sort();
    found
}

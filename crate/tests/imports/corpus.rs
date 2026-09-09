//! Which modules of a corpus a sweep runs, meaning the interpreter owning
//! the corpus and the version it reports, the entry points a run leaves out,
//! the modules a format run rewrote, and the digest that identifies the corpus
//! a run swept.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{common::python_files, records::Corpus};

/// How many characters of the corpus digest name one sweep's corpus.
const DIGEST_LEN: usize = 16;

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

/// The modules a sweep runs, which is every importable module the format
/// run rewrote outside the entry points, sorted.
pub(crate) fn candidates(rewritten: &BTreeSet<String>) -> Vec<String> {
    rewritten
        .iter()
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

/// Digests the corpus at `root` into an identity, reading each swept file's
/// path and bytes in sorted order and framing both behind their own length, so
/// a file that moves, changes, or arrives changes the digest.
pub(crate) fn identity(root: &Path, interpreter: &str) -> Corpus {
    let files: BTreeSet<_> = python_files(root).collect();
    let mut hasher = blake3::Hasher::new();
    for path in &files {
        let relative = path.strip_prefix(root).unwrap_or(path);
        framed(&mut hasher, relative.to_string_lossy().as_bytes());
        framed(&mut hasher, &fs_err::read(path).unwrap_or_default());
    }
    Corpus {
        digest: hasher.finalize().to_hex()[..DIGEST_LEN].to_owned(),
        files: files.len(),
        interpreter: interpreter.to_owned(),
    }
}

/// Reports whether a file names a module an import can bind. A `.pyw`
/// script and a `.pyi` stub both carry Python the formatter rewrites without
/// naming an importable module, so the walk formats them and this run leaves
/// them out.
pub(crate) fn importable(relative: &str) -> bool {
    relative.ends_with(".py")
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

/// Feeds `bytes` to the digest behind its own length, so two fields cannot run
/// together and read as one.
fn framed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    let len = u64::try_from(bytes.len()).expect("a corpus file's length fits a u64");
    hasher.update(&len.to_le_bytes());
    hasher.update(bytes);
}

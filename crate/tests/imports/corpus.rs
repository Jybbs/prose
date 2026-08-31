//! Which modules of a corpus a sweep runs, meaning the interpreter owning
//! the corpus, the entry points a run leaves out, and the modules a format
//! run rewrote.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use ignore::WalkBuilder;
use itertools::Itertools;

/// The modules a walk leaves out, since running an entry point launches
/// whatever it launches.
const ENTRY_POINTS: [&str; 3] = ["antigravity.py", "idlelib/idle.py", "webbrowser.py"];

/// The directories a walk leaves out wholesale.
const ENTRY_TREES: [&str; 4] = ["idle_test", "test", "tests", "turtledemo"];

/// The modules a sweep runs, which is every module the two trees differ on
/// outside the entry points, sorted.
pub(crate) fn candidates(formatted: &Path, original: &Path) -> Vec<String> {
    modules_under(original)
        .into_iter()
        .filter(|relative| {
            if excluded(relative) {
                return false;
            }
            match (
                fs_err::read(original.join(relative)),
                fs_err::read(formatted.join(relative)),
            ) {
                (Ok(was), Ok(now)) => was != now,
                _ => true,
            }
        })
        .collect()
}

/// Reports whether a module is an entry point rather than a library module.
pub(crate) fn excluded(relative: &str) -> bool {
    let parts: Vec<_> = relative.split('/').collect();
    let (last, directories) = parts.split_last().expect("a path holds a last segment");
    *last == "__main__.py"
        || ENTRY_POINTS.contains(&relative)
        || directories.iter().any(|part| ENTRY_TREES.contains(part))
}

/// Asks an interpreter which standard library it owns.
pub(crate) fn interpreter(python: &str) -> PathBuf {
    let asked = Command::new(python)
        .args([
            "-I",
            "-c",
            "import sysconfig; print(sysconfig.get_paths()['stdlib'])",
        ])
        .output()
        .unwrap_or_else(|error| panic!("{python} does not run: {error}"));
    assert!(asked.status.success(), "{python} does not run");
    PathBuf::from(String::from_utf8_lossy(&asked.stdout).trim())
        .canonicalize()
        .expect("the interpreter names a standard library")
}

/// The `.py` files under `tree`, each named relative to it, sorted.
pub(crate) fn modules_under(tree: &Path) -> Vec<String> {
    WalkBuilder::new(tree)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .filter_map(|path| Some(path.strip_prefix(tree).ok()?.to_str()?.to_owned()))
        .sorted()
        .collect()
}

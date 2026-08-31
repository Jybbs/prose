//! Which modules of a corpus a sweep runs, meaning the interpreter owning
//! the corpus, the entry points a run leaves out, and the modules a format
//! run rewrote.

use std::{collections::BTreeSet, path::PathBuf, process::Command};

/// The modules a walk leaves out, since running an entry point launches
/// whatever it launches.
const ENTRY_POINTS: &[&str] = &["antigravity.py", "idlelib/idle.py", "webbrowser.py"];

/// The directories a walk leaves out wholesale.
const ENTRY_TREES: &[&str] = &["idle_test", "test", "tests", "turtledemo"];

/// The modules a sweep runs, which is every module the format run rewrote
/// outside the entry points, sorted.
pub(crate) fn candidates(rewritten: &BTreeSet<String>) -> Vec<String> {
    rewritten
        .iter()
        .filter(|relative| !excluded(relative))
        .cloned()
        .collect()
}

/// Reports whether a module is an entry point rather than a library module.
pub(crate) fn excluded(relative: &str) -> bool {
    let (directories, last) = relative.rsplit_once('/').unwrap_or(("", relative));
    last == "__main__.py"
        || ENTRY_POINTS.contains(&relative)
        || directories
            .split('/')
            .any(|part| ENTRY_TREES.contains(&part))
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
    assert!(
        asked.status.success(),
        "{python} does not run: {}",
        String::from_utf8_lossy(&asked.stderr).trim()
    );
    PathBuf::from(String::from_utf8_lossy(&asked.stdout).trim())
        .canonicalize()
        .expect("the interpreter names a standard library")
}

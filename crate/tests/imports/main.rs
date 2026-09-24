//! The import sweep, which runs every module of a corpus from the original
//! tree and from the formatted one, then compares the two namespaces, so a
//! rewrite that settles and still breaks the code is caught. Each module runs
//! in a fresh interpreter through `probe.py`, and each break is blamed on the
//! rules whose recorded fixes reach it. The run is ignored by default because
//! it executes a corpus, and it fails on any break and on any condition that
//! leaves it unable to judge whether a module broke.

#[path = "../common/mod.rs"]
mod common;

mod attribution;
mod bindings;
mod compare;
mod corpus;
mod diff;
mod execute;
mod fixes;
mod format;
mod gate;
mod outcome;
mod reach;
mod records;
mod removed;
mod report;
mod stage;
mod sweep;

use std::{iter, num::NonZeroUsize};

use crate::{
    common::{watch_for_a_runaway, widths_or},
    corpus::{identity, interpreter, standard_library, target, version},
    gate::failures,
    report::render,
    sweep::Sweep,
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[test]
#[ignore = "the sweep executes a corpus and runs in its own row"]
fn every_module_still_imports() {
    watch_for_a_runaway();
    let python = interpreter();
    let corpus = standard_library(&python);
    let release = version(&python);
    let target = target(&release);
    let swept = identity(&corpus);
    let budgets = iter::once(None).chain(widths_or(&[]).into_iter().map(NonZeroUsize::new));
    let sweep = Sweep::new(&corpus, &python, target);
    eprintln!(
        "corpus      {} ({} files, vendored {})\nbinary      the library under test\ninterpreter \
         {python} ({})\ntarget      {target}\nstage       {}",
        corpus.display(),
        swept.files,
        swept.vendored.join(" "),
        release,
        sweep.runner.stage.root.display(),
    );
    let mut failed = Vec::new();
    for found in budgets.map(|width| sweep.sweep(width)) {
        eprintln!("\nwidth {}\n{}", found.label, render(&found));
        failed.extend(
            failures(&found)
                .into_iter()
                .map(|cause| format!("width {}, {cause}", found.label)),
        );
    }
    if !failed.is_empty() {
        sweep.runner.stage.keep();
    }
    assert!(
        failed.is_empty(),
        "the run fails:\n  {}",
        failed.join("\n  ")
    );
}

#[cfg(test)]
mod tests;

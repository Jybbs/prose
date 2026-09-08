//! The import sweep, which runs every module the formatter rewrote from the
//! original tree and from the formatted one, then compares the two
//! namespaces, so a rewrite that settles and still breaks the code is
//! caught. Each module runs in a fresh interpreter through `probe.py`, and
//! each break is blamed on the rules whose recorded fixes reach it. The run
//! is ignored by default because it executes a corpus, and it ratchets
//! against the break set tracked beside this harness.

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
mod outcome;
mod ratchet;
mod records;
mod report;
mod stage;
mod sweep;

use std::{collections::BTreeSet, iter, num::NonZeroUsize};

use crate::{
    common::{watch_for_a_runaway, widths_or},
    corpus::standard_library,
    execute::interpreter,
    ratchet::{bake, baking, baseline, dropped, judge, skipping},
    report::render,
    sweep::{Sweep, label},
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[test]
#[ignore = "the sweep executes a corpus and runs in its own row"]
fn every_rewritten_module_still_imports() {
    watch_for_a_runaway();
    let python = interpreter();
    let corpus = standard_library(&python);
    let baked = baking();
    let held = baked.is_none().then(baseline).unwrap_or_default();
    let budgets = iter::once(None).chain(widths_or(&[]).into_iter().map(NonZeroUsize::new));
    let sweep = Sweep::new(&corpus);
    eprintln!(
        "corpus      {}\nbinary      the library under test\ninterpreter {python}\nstage       {}",
        corpus.display(),
        sweep.runner.stage.root.display(),
    );
    let widths: Vec<_> = budgets
        .map(|width| sweep.sweep(width, skipping(&held, &label(width))))
        .collect();
    let mut fresh = BTreeSet::new();
    let mut lost = BTreeSet::new();
    for found in &widths {
        let carried = judge(found, &held);
        lost.extend(dropped(found, &held));
        eprintln!("\nwidth {}\n{}", found.label, render(&carried, found));
        fresh.extend(found.uncarried(&carried).map(|brk| brk.module.clone()));
    }
    let unmeasured: usize = widths.iter().map(|found| found.unmeasured.len()).sum();
    if unmeasured > 0 {
        sweep.runner.stage.keep();
    }
    assert!(
        unmeasured == 0,
        "the run leaves {unmeasured} of its modules unmeasured, so the uncomparable count cannot \
         be named",
    );
    if let Some(path) = baked {
        bake(&path, &widths);
        eprintln!("break set baked into {}", path.display());
        return;
    }
    if !lost.is_empty() || !fresh.is_empty() {
        sweep.runner.stage.keep();
    }
    assert!(
        lost.is_empty(),
        "the baseline compares {} of the modules this run could not, the first being {}",
        lost.len(),
        lost.first().map_or("", String::as_str),
    );
    assert!(
        fresh.is_empty(),
        "the baseline does not carry {} of the modules that break, the first being {}",
        fresh.len(),
        fresh.first().map_or("", String::as_str),
    );
}

#[cfg(test)]
mod tests;

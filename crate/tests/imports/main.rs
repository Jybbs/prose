//! The import sweep, wherein every module the formatter rewrites is executed
//! from the original tree and from the formatted one and the two namespaces
//! are compared, so a rewrite that settles and still breaks the code is
//! caught.
//!
//! Each module runs in a fresh interpreter through `probe.py`, which loads it
//! the way an import loads it and records the names and plain constants it
//! bound. A module counts as broken where the original runs cleanly and the
//! formatted copy raises, times out, or binds a different namespace,
//! confirmed by a second run of both sides so a module that flips reports as
//! flaky. Each break is attributed to the deepest traceback frame under the
//! formatted tree, then to the rules whose recorded fixes cover that row or
//! dropped the binding of the name it turns on, and failing both to the rules
//! reproducing it under one rule alone.
//!
//! The sweep is ignored by default, since it executes a corpus and costs a
//! minute of wall clock. `PROSE_IMPORTS_PYTHON` names the interpreter whose
//! standard library it runs, `PROSE_IMPORTS_MODULE` narrows it to one module,
//! `PROSE_SETTLE_WIDTHS` adds widths beside the default,
//! `PROSE_IMPORTS_TIMEOUT` bounds one module's run, `PROSE_IMPORTS_BAKE`
//! writes the break set, and `PROSE_IMPORTS_BASELINE` names one an earlier
//! run wrote, so only a break it does not carry fails the run or reaches
//! the report.
//!
//! A baked set carries the modules the original tree did not run cleanly
//! beside the breaks, which a judging run skips rather than paying to
//! measure again, and a module falling out of comparison that the set does
//! not list fails the run the way a fresh break does, so coverage the
//! sweep loses is caught rather than going quiet.

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
    common::{setting, watch_for_a_runaway, widths_or},
    corpus::interpreter,
    ratchet::{bake, baking, baseline, dropped, judge, skipping},
    report::render,
    sweep::{PYTHON_VAR, Sweep, label},
};

/// The interpreter the sweep runs absent [`PYTHON_VAR`].
const PYTHON: &str = "python3";

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[test]
#[ignore = "the sweep executes a corpus and runs in its own row"]
fn every_rewritten_module_still_imports() {
    watch_for_a_runaway();
    let python = setting(PYTHON_VAR).unwrap_or_else(|| PYTHON.to_owned());
    let corpus = interpreter(&python);
    let held = baseline();
    let baked = baking();
    let widths = iter::once(None).chain(widths_or(&[]).into_iter().map(NonZeroUsize::new));
    let sweep = Sweep::new(&corpus, python.clone());
    eprintln!(
        "corpus      {}\nbinary      the library under test\ninterpreter {python}\nstage       {}",
        corpus.display(),
        sweep.runner.stage.root.display(),
    );
    let found: Vec<_> = widths
        .map(|width| {
            let label = label(width);
            sweep.sweep(
                width,
                baked.is_none().then(|| skipping(&held, &label)).flatten(),
            )
        })
        .collect();
    let mut fresh = BTreeSet::new();
    let mut lost = BTreeSet::new();
    for width in &found {
        let carried = judge(width, &held);
        lost.extend(dropped(width, &held));
        eprintln!("\nwidth {}\n{}", width.label, render(&carried, width));
        fresh.extend(
            width
                .breaks
                .iter()
                .filter(|brk| !carried.contains(&brk.module))
                .map(|brk| brk.module.clone()),
        );
    }
    let unmeasured: usize = found.iter().map(|width| width.unmeasured.len()).sum();
    assert!(
        unmeasured == 0,
        "the run leaves {unmeasured} of its modules unmeasured, so the uncomparable count cannot \
         be named",
    );
    if let Some(path) = baked {
        bake(&path, &found);
        eprintln!("break set baked into {}", path.display());
        return;
    }
    assert!(
        lost.is_empty(),
        "the baseline compares {} of the modules this run could not, the first being {}",
        lost.len(),
        lost.iter().next().map_or("", String::as_str),
    );
    assert!(
        fresh.is_empty(),
        "the baseline does not carry {} of the modules that break, the first being {}",
        fresh.len(),
        fresh.iter().next().map_or("", String::as_str),
    );
}

#[cfg(test)]
mod tests;

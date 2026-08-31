//! Import sweep: every module the formatter rewrites is executed from the
//! original tree and from the formatted one, and the two namespaces are
//! compared, so a rewrite that settles and still breaks the code is caught.
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
//! `PROSE_IMPORTS_WIDTHS` adds widths beside the default,
//! `PROSE_IMPORTS_TIMEOUT` bounds one module's run, `PROSE_IMPORTS_BAKE`
//! writes the break set, and `PROSE_IMPORTS_BASELINE` names one an earlier
//! run wrote, so only a break it does not carry fails the run.

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
mod ratchet;
mod records;
mod report;
mod stage;
mod sweep;

use std::{collections::BTreeSet, env, iter, num::NonZeroUsize, path::Path};

use crate::{
    common::env_list,
    corpus::interpreter,
    ratchet::{BAKE_VAR, bake, baseline, judge},
    report::render,
    sweep::{PYTHON_VAR, Sweep, WIDTHS_VAR},
};

/// The interpreter the sweep runs absent [`PYTHON_VAR`].
const PYTHON: &str = "python3";

#[test]
#[ignore = "the sweep executes a corpus and runs in its own row"]
fn every_rewritten_module_still_imports() {
    let python = env::var(PYTHON_VAR).unwrap_or_else(|_| PYTHON.to_owned());
    let corpus = interpreter(&python);
    let sweep = Sweep::new(&corpus, python.clone());
    let widths = iter::once(None).chain(env_list(WIDTHS_VAR, &[], |width| {
        Some(
            width
                .parse::<NonZeroUsize>()
                .unwrap_or_else(|_| panic!("every `{WIDTHS_VAR}` entry is a number")),
        )
    }));
    println!(
        "corpus      {}\nbinary      the library under test\ninterpreter {python}\nstage       {}",
        corpus.display(),
        sweep.stage.root.display(),
    );
    let held = baseline();
    let found: Vec<_> = widths.map(|width| sweep.sweep(width)).collect();
    let mut fresh = BTreeSet::new();
    for width in &found {
        let carried = judge(width, &held);
        println!("\nwidth {}\n{}", width.label, render(&carried, width));
        fresh.extend(
            width
                .breaks
                .iter()
                .filter(|brk| !carried.contains(&brk.module))
                .map(|brk| brk.module.clone()),
        );
    }
    let unmeasured: Vec<_> = found
        .iter()
        .flat_map(|width| width.unmeasured.iter())
        .collect();
    assert!(
        unmeasured.is_empty(),
        "a run left {} modules unmeasured, so the uncomparable count cannot be named",
        unmeasured.len(),
    );
    if let Some(baked) = env::var_os(BAKE_VAR) {
        let baked = Path::new(&baked);
        bake(baked, &found);
        println!("break set baked into {}", baked.display());
        return;
    }
    assert!(
        fresh.is_empty(),
        "{} modules break that the baseline does not carry, the first being {}",
        fresh.len(),
        fresh.iter().next().map_or("", String::as_str),
    );
}

#[cfg(test)]
mod tests;

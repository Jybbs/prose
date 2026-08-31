//! Import sweep: every module the formatter rewrites is executed from the
//! original tree and from the formatted one, and the two namespaces are
//! compared, so a rewrite that settles and still breaks the code is caught.
//! A settling gate asks whether a second pass changes the output, which
//! output that is broken and stable passes exactly as correct output does,
//! and no static reader closes the gap, because a name bound in class scope,
//! a `del`, and a member shadowing a module global each defeat a binding
//! scan.
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

use std::{env, num::NonZeroUsize, path::Path};

use common::env_list;
use corpus::interpreter;
use itertools::Itertools;
use ratchet::{BAKE_VAR, bake, baseline, judge};
use report::render;
use sweep::{PYTHON_VAR, Sweep, WIDTHS_VAR};

/// The environment variable naming a file every break is written to, one
/// row apiece, which is what a comparison against another harness reads.
const ROWS_VAR: &str = "PROSE_IMPORTS_ROWS";

/// The interpreter the sweep runs absent [`PYTHON_VAR`].
const PYTHON: &str = "python3";

#[test]
#[ignore = "the sweep executes a corpus and runs in its own row"]
fn every_rewritten_module_still_imports() {
    let python = env::var(PYTHON_VAR).unwrap_or_else(|_| PYTHON.to_owned());
    let corpus = interpreter(&python);
    let sweep = Sweep::new(&corpus, python.clone());
    let widths = std::iter::once(None).chain(env_list(WIDTHS_VAR, &[], |width| {
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
    let mut fresh = Vec::new();
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
    if let Some(rows) = env::var_os(ROWS_VAR) {
        let listed: Vec<_> = found
            .iter()
            .flat_map(|width| {
                width.breaks.iter().map(|brk| {
                    let (file, row) = &brk.frame;
                    let at = row.map_or_else(String::new, |row| row.to_string());
                    format!(
                        "{}\t{}\t{}\t{at}\t{}",
                        width.label, brk.module, file, brk.reason
                    )
                })
            })
            .sorted()
            .collect();
        fs_err::write(Path::new(&rows), listed.join("\n") + "\n").expect("write the break rows");
        println!(
            "\n{} break rows written to {}",
            listed.len(),
            Path::new(&rows).display()
        );
    }
    println!("\neach tree survives under {}", sweep.stage.root.display());
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
        bake(Path::new(&baked), &found);
        println!("break set baked into {}", Path::new(&baked).display());
        return;
    }
    assert!(
        fresh.is_empty(),
        "{} modules break that the baseline does not carry, the first being {}",
        fresh.len(),
        fresh[0],
    );
}

#[cfg(test)]
mod tests;

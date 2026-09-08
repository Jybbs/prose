//! One sweep of a corpus at a width, meaning a formatted copy, the run of
//! every module the formatter rewrote from both trees, and each break
//! confirmed and attributed.

use std::{
    collections::BTreeMap,
    num::NonZeroUsize,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use prose::{config::Config, pipeline::Pipeline};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    attribution::Attributor,
    common::setting,
    compare::{compare, divergence},
    corpus::candidates,
    execute::Runner,
    format::format_tree,
    outcome::{Kind, Outcome},
    records::{Break, Width},
};

/// The label a sweep gives the width no `code-line-length` pinned.
pub(crate) const DEFAULT_LABEL: &str = "default";

/// The environment variable narrowing a run to one module.
pub(crate) const MODULE_VAR: &str = "PROSE_IMPORTS_MODULE";

/// What a second run of both sides made of a break.
enum Confirmed {
    /// Both reruns agree the break is real.
    Break,
    /// The original disagrees with its own rerun, so nothing is proven.
    Flaky,
    /// The formatted rerun left no record, so the break is unmeasured.
    Unmeasured,
}

/// One corpus, the runner every module goes through, and the outcomes
/// already read from the original tree.
pub(crate) struct Sweep {
    /// What the original tree left for each module already run from it,
    /// which every width reads rather than running the tree again.
    known: Mutex<BTreeMap<String, Outcome>>,
    /// The interpreter, deadline, and stage every run goes through.
    pub(crate) runner: Runner,
}

impl Sweep {
    /// Builds the sweep, copying the corpus into a fresh stage.
    pub(crate) fn new(corpus: &Path) -> Self {
        Self {
            known: Mutex::new(BTreeMap::new()),
            runner: Runner::new(corpus),
        }
    }

    /// What a second run of both sides makes of a break, reading
    /// `Unmeasured` where the formatted rerun left no record so a lost
    /// record does not read as flake.
    fn confirm(&self, brk: &Break, formatted: &Path) -> Confirmed {
        let before = self
            .runner
            .run(&brk.module, &[self.runner.stage.original.as_path()]);
        if before.kind != Kind::Ok || divergence(&before, &brk.original).is_some() {
            return Confirmed::Flaky;
        }
        let after = self.runner.run(&brk.module, &[formatted]);
        if after.kind == Kind::Unmeasured {
            return Confirmed::Unmeasured;
        }
        if divergence(&after, &before).is_some() {
            Confirmed::Break
        } else {
            Confirmed::Flaky
        }
    }

    /// The memo of what the original tree left for each module it has
    /// already been asked about.
    fn memo(&self) -> MutexGuard<'_, BTreeMap<String, Outcome>> {
        self.known.lock().expect("the memo is never poisoned")
    }

    /// Runs the modules the original tree has not yet been asked about and
    /// returns what it left for every one of them.
    fn originals(&self, modules: &[String]) -> BTreeMap<String, Outcome> {
        let missing: Vec<_> = {
            let known = self.memo();
            modules
                .iter()
                .filter(|module| !known.contains_key(*module))
                .cloned()
                .collect()
        };
        let ran = self.outcomes(&missing, &self.runner.stage.original);
        let mut known = self.memo();
        known.extend(ran);
        modules
            .iter()
            .filter_map(|module| Some((module.clone(), known.get(module)?.clone())))
            .collect()
    }

    /// Runs every one of some modules from one tree, sharing the worker pool.
    fn outcomes(&self, modules: &[String], tree: &Path) -> BTreeMap<String, Outcome> {
        modules
            .par_iter()
            .map(|module| (module.clone(), self.runner.run(module, &[tree])))
            .collect()
    }

    /// Sweeps the corpus at one width, running every module the formatter
    /// rewrote from both trees and confirming each break by a second run.
    pub(crate) fn sweep(&self, width: Option<NonZeroUsize>) -> Width {
        let label = label(width);
        let config = width.map_or_else(Config::default, |width| Config {
            code_line_length: Some(width),
            ..Config::default()
        });
        let formatted = self.runner.stage.copy(&format!("formatted-{label}"));
        let run = format_tree(&formatted, &Pipeline::with_defaults(&config));
        self.runner.precompile(&formatted);
        let modules =
            setting(MODULE_VAR).map_or_else(|| candidates(&run.rewritten), |only| vec![only]);
        let after = self.outcomes(&modules, &formatted);
        let before = self.originals(&modules);
        let partition = compare(&after, &before, &modules);
        let judged: Vec<_> = partition
            .breaks
            .into_par_iter()
            .map(|brk| (self.confirm(&brk, &formatted), brk))
            .collect();
        let mut breaks = Vec::new();
        let mut flaky = Vec::new();
        let mut unmeasured = partition.unmeasured;
        for (verdict, brk) in judged {
            match verdict {
                Confirmed::Break => breaks.push(brk),
                Confirmed::Flaky => flaky.push(brk.module),
                Confirmed::Unmeasured => unmeasured.push(brk.module),
            }
        }
        Attributor {
            config: &config,
            fixes: &run.fixes,
            formatted: &formatted,
            label: &label,
            runner: &self.runner,
        }
        .attribute(&mut breaks);
        Width {
            breaks,
            candidates: modules.len(),
            comparable: partition.comparable,
            flaky,
            label,
            refused: run.refused,
            uncomparable: partition.uncomparable,
            unmeasured,
        }
    }
}

/// The label one width is keyed by, which the bake, the ratchet, and the
/// report all read.
pub(crate) fn label(width: Option<NonZeroUsize>) -> String {
    width.map_or_else(|| DEFAULT_LABEL.to_owned(), |width| width.to_string())
}

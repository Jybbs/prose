//! One sweep of a corpus at a width, meaning a formatted copy, the run of
//! every module of the corpus from both trees, the comparison that sets
//! aside each name a recorded fix removed, and each break confirmed and
//! attributed.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use prose::{config::Config, pipeline::Pipeline};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ruff_python_ast::PythonVersion;

use crate::{
    attribution::Attributor,
    common::setting,
    compare::{compare, divergence, varying},
    corpus::{candidates, excluded, importable},
    execute::Runner,
    format::format_tree,
    outcome::{Kind, Outcome},
    records::{Break, Width},
    removed::removed,
};

/// The label a sweep gives the width no `code-line-length` pinned.
pub(crate) const DEFAULT_LABEL: &str = "default";

/// The environment variable narrowing a run to one module.
const MODULE_VAR: &str = "PROSE_IMPORTS_MODULE";

/// What a second run of both sides made of a break, each measured variant
/// carrying the names the original's two runs bound differently.
#[derive(Debug)]
pub(crate) enum Confirmed {
    /// The two reruns still diverge past those names, so the break is real.
    Break(BTreeSet<String>),
    /// The two reruns agree once those names are set aside, so the variance
    /// accounts for the whole difference. An empty set stands for a module
    /// whose entire namespace varied, which a run sets aside rather than
    /// narrowing.
    Flaky(BTreeSet<String>),
    /// A rerun left no record, so nothing about the module is measured.
    Unmeasured,
}

/// One corpus, the runner every module goes through, the outcomes already
/// read from the original tree, and the version every format run targets.
pub(crate) struct Sweep {
    /// What the original tree left for each module already run from it,
    /// which every width reads rather than running the tree again.
    known: Mutex<BTreeMap<String, Outcome>>,
    /// The interpreter, deadline, and stage every run goes through.
    pub(crate) runner: Runner,
    /// The version every format run targets.
    target: PythonVersion,
}

impl Sweep {
    /// Builds the sweep over `python`, copying the corpus into a fresh stage,
    /// every format run targeting `target`.
    pub(crate) fn new(corpus: &Path, python: &str, target: PythonVersion) -> Self {
        Self {
            known: Mutex::new(BTreeMap::new()),
            runner: Runner::new(corpus, python.to_owned()),
            target,
        }
    }

    /// What a second run of both sides makes of a break, running the
    /// original again and then the formatted copy, each with `aside` left
    /// out.
    fn confirm(&self, brk: &Break, formatted: &Path, aside: &BTreeSet<String>) -> Confirmed {
        let before = self
            .runner
            .run(&brk.module, &[self.runner.stage.original.as_path()])
            .without(aside);
        if let Some(reached) = settled(before.kind) {
            return reached;
        }
        let varies = varying(&before, &brk.original);
        let after = self.runner.run(&brk.module, &[formatted]).without(aside);
        verdict(&after, &before, varies)
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

    /// Sweeps the corpus at one width, running every module of the corpus
    /// from both trees, leaving out each name a recorded fix removed, and
    /// confirming each break by a second run.
    pub(crate) fn sweep(&self, width: Option<NonZeroUsize>) -> Width {
        let label = label(width);
        let defaults = Config::default();
        let config = Config {
            code_line_length: width.or(defaults.code_line_length),
            target_version: Some(self.target),
            ..defaults
        };
        let formatted = self.runner.stage.copy(&format!("formatted-{label}"));
        let run = format_tree(&formatted, &Pipeline::with_defaults(&config));
        self.runner.precompile(&formatted);
        let modules = setting(MODULE_VAR).map_or_else(
            || candidates(&run.read),
            |only| {
                assert!(
                    importable(&only) && !excluded(&only),
                    "{MODULE_VAR} names {only}, which the sweep never runs, because it is an \
                     entry point or names no module an import can bind",
                );
                vec![only]
            },
        );
        let mut after = self.outcomes(&modules, &formatted);
        let mut before = self.originals(&modules);
        let removed = removed(&after, &before, &run.fixes, &self.runner.stage.original);
        let aside: BTreeMap<String, BTreeSet<String>> = removed
            .iter()
            .map(|(module, names)| (module.clone(), names.keys().cloned().collect()))
            .collect();
        for (module, names) in &aside {
            for held in [&mut after, &mut before] {
                if let Some(ran) = held.get_mut(module) {
                    *ran = ran.without(names);
                }
            }
        }
        let partition = compare(&after, &before, &modules);
        let none = BTreeSet::new();
        let judged: Vec<_> = partition
            .breaks
            .into_par_iter()
            .map(|brk| {
                let names = aside.get(&brk.module).unwrap_or(&none);
                (self.confirm(&brk, &formatted, names), brk)
            })
            .collect();
        let mut breaks = Vec::new();
        let mut flaky = BTreeMap::new();
        let mut unmeasured = partition.unmeasured;
        for (verdict, brk) in judged {
            match verdict {
                Confirmed::Break(varies) => {
                    if !varies.is_empty() {
                        flaky.insert(brk.module.clone(), varies);
                    }
                    breaks.push(brk);
                }
                Confirmed::Flaky(varies) => {
                    flaky.insert(brk.module, varies);
                }
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
            rejected: run.rejected,
            removed,
            rewritten: run.rewritten,
            uncomparable: partition.uncomparable,
            unmeasured,
            unread: run.unread,
        }
    }
}

/// The verdict a rerun of the original reaches on its own, `None` where it
/// ran cleanly and the formatted side is worth running. A rerun that timed
/// out or left no record measures nothing, and one that raised bound no
/// namespace to compare, so its variance carries no names.
pub(crate) fn settled(kind: Kind) -> Option<Confirmed> {
    match kind {
        Kind::Ok => None,
        Kind::Raised => Some(Confirmed::Flaky(BTreeSet::new())),
        Kind::Timeout | Kind::Unmeasured => Some(Confirmed::Unmeasured),
    }
}

/// The verdict two reruns reach once `varies` is set aside, which is a
/// break wherever they still diverge on some name outside it. A formatted
/// rerun that left no record measures nothing, whatever the first pair
/// showed.
pub(crate) fn verdict(after: &Outcome, before: &Outcome, varies: BTreeSet<String>) -> Confirmed {
    if after.kind == Kind::Unmeasured {
        return Confirmed::Unmeasured;
    }
    if divergence(&after.without(&varies), &before.without(&varies)).is_some() {
        Confirmed::Break(varies)
    } else {
        Confirmed::Flaky(varies)
    }
}

/// The label one width goes by in the report, in the run's failures, and in
/// the names of the stage's directories.
fn label(width: Option<NonZeroUsize>) -> String {
    width.map_or_else(|| DEFAULT_LABEL.to_owned(), |width| width.to_string())
}

//! One sweep of a corpus at a width, meaning a formatted copy, the run of
//! every module the formatter rewrote from both trees, and each break
//! confirmed and attributed.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    path::Path,
    sync::Mutex,
};

use itertools::{Either, Itertools};
use prose::{config::Config, pipeline::Pipeline};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    attribution::Attributor,
    common::setting,
    compare::{compare, divergence},
    corpus::candidates,
    execute::Runner,
    fixes::drops,
    format::format_tree,
    outcome::{Kind, Outcome},
    records::{Break, Fixes, Width},
};

/// The label a sweep gives the width no `code-line-length` pinned.
pub(crate) const DEFAULT_LABEL: &str = "default";

/// The environment variable narrowing a run to one module.
const MODULE_VAR: &str = "PROSE_IMPORTS_MODULE";

/// The environment variable naming the interpreter whose standard library
/// the sweep runs.
pub(crate) const PYTHON_VAR: &str = "PROSE_IMPORTS_PYTHON";

/// One corpus, the runner every module goes through, and what the original
/// tree has already been asked.
pub(crate) struct Sweep {
    /// What the original tree left for each module already run from it,
    /// which every width reads rather than running the tree again.
    known: Mutex<BTreeMap<String, Outcome>>,
    /// The interpreter, deadline, and stage every run goes through.
    pub(crate) runner: Runner,
}

impl Sweep {
    /// Builds the sweep, copying the corpus into a fresh stage.
    pub(crate) fn new(corpus: &Path, python: String) -> Self {
        Self {
            known: Mutex::new(BTreeMap::new()),
            runner: Runner::new(corpus, python),
        }
    }

    /// Reports whether a break is one module losing a name a recorded fix
    /// deliberately dropped from that same module, which is a rule doing
    /// its work rather than a rewrite breaking the code. A module that
    /// reads the dropped name still raises and still counts.
    fn deliberately_pruned(&self, brk: &Break, fixes: &Fixes) -> bool {
        brk.formatted.kind == Kind::Ok
            && brk.reason.ends_with("` unbound")
            && brk.name.as_deref().is_some_and(|name| {
                let text = fs_err::read_to_string(self.runner.stage.original.join(&brk.module))
                    .unwrap_or_default();
                fixes
                    .get(&brk.module)
                    .is_some_and(|listed| listed.iter().any(|(_, edits)| drops(edits, name, &text)))
            })
    }

    /// Reports whether the original matches its own first run and a second
    /// run of the formatted side still breaks.
    fn confirm(&self, brk: &Break, formatted: &Path) -> bool {
        let before = self
            .runner
            .run(&brk.module, &[self.runner.stage.original.as_path()]);
        if before.kind != Kind::Ok || divergence(&before, &brk.original).is_some() {
            return false;
        }
        let after = self.runner.run(&brk.module, &[formatted]);
        after.kind != Kind::Unmeasured && divergence(&after, &before).is_some()
    }

    /// Runs the modules the original tree has not yet been asked about and
    /// returns what it left for every one of them.
    fn originals(&self, modules: &[String]) -> BTreeMap<String, Outcome> {
        let missing: Vec<_> = {
            let known = self.known.lock().expect("the memo is never poisoned");
            modules
                .iter()
                .filter(|module| !known.contains_key(*module))
                .cloned()
                .collect()
        };
        let ran = self.outcomes(&missing, &self.runner.stage.original);
        let mut known = self.known.lock().expect("the memo is never poisoned");
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
    pub(crate) fn sweep(
        &self,
        width: Option<NonZeroUsize>,
        skip: Option<&BTreeSet<String>>,
    ) -> Width {
        let label = width.map_or_else(|| DEFAULT_LABEL.to_owned(), |width| width.to_string());
        let config = width.map_or_else(Config::default, |width| Config {
            code_line_length: Some(width),
            ..Config::default()
        });
        let formatted = self.runner.stage.copy(&format!("formatted-{label}"));
        let run = format_tree(&formatted, &Pipeline::with_defaults(&config));
        let modules = setting(MODULE_VAR).map_or_else(
            || {
                let found = candidates(&run.rewritten);
                match skip {
                    Some(held) => found
                        .into_iter()
                        .filter(|module| !held.contains(module))
                        .collect(),
                    None => found,
                }
            },
            |only| vec![only],
        );
        let after = self.outcomes(&modules, &formatted);
        let before = self.originals(&modules);
        let partition = compare(&after, &before, &modules);
        let (suspects, pruned): (Vec<_>, Vec<_>) =
            partition.breaks.into_iter().partition_map(|brk| {
                if self.deliberately_pruned(&brk, &run.fixes) {
                    Either::Right(brk.module)
                } else {
                    Either::Left(brk)
                }
            });
        let verdicts: Vec<_> = suspects
            .par_iter()
            .map(|brk| self.confirm(brk, &formatted))
            .collect();
        let (mut breaks, flaky): (Vec<_>, Vec<_>) = suspects
            .into_iter()
            .zip(verdicts)
            .partition_map(|(brk, holds)| {
                if holds {
                    Either::Left(brk)
                } else {
                    Either::Right(brk.module)
                }
            });
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
            pruned,
            refused: run.refused,
            uncomparable: partition.uncomparable,
            unmeasured: partition.unmeasured,
        }
    }
}

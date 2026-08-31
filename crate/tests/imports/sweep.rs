//! One sweep of a corpus at a width, meaning a formatted copy, the run of
//! every module the formatter rewrote from both trees, and each break
//! confirmed and attributed.

use std::{collections::BTreeMap, env, num::NonZeroUsize, path::Path, sync::Mutex};

use prose::{config::Config, pipeline::Pipeline};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    attribution::Attributor,
    compare::{compare, divergence},
    corpus::candidates,
    execute::execute,
    format::{REFUSED, format_tree},
    records::{Outcome, Width},
    stage::Stage,
};

/// The environment variable narrowing a run to one module.
pub(crate) const MODULE_VAR: &str = "PROSE_IMPORTS_MODULE";

/// The environment variable naming the interpreter whose standard library
/// the sweep runs.
pub(crate) const PYTHON_VAR: &str = "PROSE_IMPORTS_PYTHON";

/// The environment variable bounding one module's run, in seconds.
pub(crate) const TIMEOUT_VAR: &str = "PROSE_IMPORTS_TIMEOUT";

/// The environment variable adding widths beside the default.
pub(crate) const WIDTHS_VAR: &str = "PROSE_IMPORTS_WIDTHS";

/// How many seconds one module may run for absent [`TIMEOUT_VAR`].
const TIMEOUT: f64 = 30.0;

/// One corpus, the interpreter owning it, and the stage a sweep works
/// through.
pub(crate) struct Sweep {
    /// What the original tree left for each module already run from it,
    /// which every width reads rather than running the tree again.
    known: Mutex<BTreeMap<String, Outcome>>,
    /// The interpreter each module runs under.
    pub(crate) python: String,
    /// How many seconds one module may run for.
    pub(crate) seconds: f64,
    /// The scratch stage every copy and run lives in.
    pub(crate) stage: Stage,
}

impl Sweep {
    /// Builds the sweep, copying the corpus into a fresh stage.
    pub(crate) fn new(corpus: &Path, python: String) -> Self {
        Self {
            known: Mutex::new(BTreeMap::new()),
            python,
            seconds: env::var(TIMEOUT_VAR)
                .ok()
                .and_then(|held| held.parse().ok())
                .unwrap_or(TIMEOUT),
            stage: Stage::new(corpus),
        }
    }

    /// Sweeps the corpus at one width, running every module the formatter
    /// rewrote from both trees and confirming each break by a second run.
    pub(crate) fn sweep(&self, width: Option<NonZeroUsize>) -> Width {
        let label = width.map_or_else(|| "default".to_owned(), |width| width.to_string());
        let config = width.map_or_else(Config::default, |width| Config {
            code_line_length: Some(width),
            ..Config::default()
        });
        let formatted = self.stage.copy(&format!("formatted-{label}"));
        let fixes = format_tree(&formatted, &Pipeline::with_defaults(&config));
        let refused = REFUSED.load(std::sync::atomic::Ordering::Relaxed);
        let modules = match env::var(MODULE_VAR) {
            Ok(only) if !only.is_empty() => vec![only],
            _ => candidates(&formatted, &self.stage.original),
        };
        let after = self.outcomes(&modules, &formatted);
        let before = self.originals(&modules);
        let (suspects, comparable, unmeasured) = compare(&after, &before, &modules);
        let verdicts: Vec<_> = suspects
            .par_iter()
            .map(|brk| self.confirm(brk, &formatted))
            .collect();
        let mut breaks = Vec::new();
        let mut flaky = Vec::new();
        for (brk, holds) in suspects.into_iter().zip(verdicts) {
            if holds {
                breaks.push(brk);
            } else {
                flaky.push(brk.module);
            }
        }
        Attributor {
            config: &config,
            fixes: &fixes,
            formatted: &formatted,
            label: &label,
            python: &self.python,
            seconds: self.seconds,
            stage: &self.stage,
        }
        .attribute(&mut breaks);
        Width {
            breaks,
            candidates: modules.len(),
            comparable,
            flaky,
            label,
            refused,
            unmeasured,
        }
    }

    /// Reports whether the original matches its own first run and a second
    /// run of the formatted side still breaks.
    fn confirm(&self, brk: &crate::records::Break, formatted: &Path) -> bool {
        let before = self.run(&brk.module, &[self.stage.original.as_path()]);
        before.kind == "ok"
            && divergence(&before, &brk.original).is_none()
            && divergence(&self.run(&brk.module, &[formatted]), &before).is_some()
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
        let ran = self.outcomes(&missing, &self.stage.original);
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
            .map(|module| (module.clone(), self.run(module, &[tree])))
            .collect()
    }

    /// Runs one module from the given trees.
    fn run(&self, module: &str, trees: &[&Path]) -> Outcome {
        execute(&self.stage, &self.python, module, trees, self.seconds)
    }
}

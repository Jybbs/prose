//! Attributing a break, meaning the frame it raises in, the rules whose
//! recorded fixes reach that frame or drop the binding it turns on, and the
//! rules reproducing it under one rule alone where no record does.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use itertools::Itertools;
use prose::{config::Config, pipeline::Pipeline, rule::render_slugs};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    bindings::binding_rows,
    compare::divergence,
    diff::{hunk, mapped_rows},
    execute::execute,
    fixes::{drops, reaches},
    format::format_tree,
    records::{Break, EditRows, Fixes},
    stage::Stage,
};

/// One width's formatted tree and the fixes its format run recorded, with
/// what an attribution re-formats and re-runs through.
pub(crate) struct Attributor<'a> {
    /// The configuration a single-rule re-format runs under.
    pub(crate) config: &'a Config,
    /// The format run's fixes, grouped by the file each rewrote.
    pub(crate) fixes: &'a Fixes,
    /// The formatted tree the breaks were found against.
    pub(crate) formatted: &'a Path,
    /// The width's label, which separates one sweep's overlays from another's.
    pub(crate) label: &'a str,
    /// The interpreter a re-run goes through.
    pub(crate) python: &'a str,
    /// How many seconds one module may run for.
    pub(crate) seconds: f64,
    /// The scratch stage every overlay and run lives in.
    pub(crate) stage: &'a Stage,
}

impl Attributor<'_> {
    /// Re-formats every module a break's run loaded under each rule on its
    /// own and returns the rules reproducing it for the same reason, joined
    /// in pipeline order.
    fn alone(&self, brk: &Break) -> String {
        let loaded = brk.loaded();
        let reproducing: Vec<_> = Pipeline::known_ids()
            .par_iter()
            .filter(|rule| {
                let slug = rule.as_str();
                let Some(pipeline) = Pipeline::for_rule(slug, self.config) else {
                    return false;
                };
                let tree = self.stage.overlay(&loaded, self.label, &brk.module, slug);
                format_tree(&tree, &pipeline);
                let ran = execute(
                    self.stage,
                    self.python,
                    &brk.module,
                    &[&tree, &self.stage.original],
                    self.seconds,
                );
                divergence(&ran, &brk.original).is_some_and(|(why, _)| why == brk.reason)
            })
            .copied()
            .collect();
        render_slugs(&reproducing).to_string()
    }

    /// Locates every break and explains the first of each group sharing a
    /// frame and reason, the rest taking that one's attribution and hunk.
    pub(crate) fn attribute(&self, breaks: &mut [Break]) {
        for brk in breaks.iter_mut() {
            brk.frame = self.locate(brk);
        }
        let mut leaders: BTreeMap<(String, Option<usize>, String), usize> = BTreeMap::new();
        let mut follows: Vec<Option<usize>> = Vec::with_capacity(breaks.len());
        for (at, brk) in breaks.iter().enumerate() {
            let key = (brk.frame.0.clone(), brk.frame.1, brk.reason.clone());
            let leader = *leaders.entry(key).or_insert(at);
            follows.push((leader != at).then_some(leader));
        }
        let ordered: Vec<_> = leaders.values().copied().sorted().collect();
        let explained: Vec<_> = ordered
            .par_iter()
            .map(|at| self.explain(&breaks[*at]))
            .collect();
        for (at, (attribution, lines)) in ordered.iter().zip(explained) {
            breaks[*at].attribution = attribution;
            breaks[*at].hunk = lines;
        }
        for at in 0..breaks.len() {
            if let Some(leader) = follows[at] {
                breaks[at].attribution = breaks[leader].attribution.clone();
                breaks[at].hunk = breaks[leader].hunk.clone();
            }
        }
    }

    /// The clause naming where the name a break turns on was bound and the
    /// rules whose fixes dropped it, empty where no fix did.
    fn binding(&self, brk: &Break) -> String {
        let name = brk.name.as_deref().unwrap_or_default();
        for module in brk.loaded() {
            let path = self.stage.original.join(&module);
            let Ok(text) = fs_err::read_to_string(&path) else {
                continue;
            };
            let Some(rows) = binding_rows(&text).get(name).cloned() else {
                continue;
            };
            let listed = self.fitting(&module, |edits| {
                reaches(edits, &rows, "") && drops(edits, name, &text)
            });
            if !listed.is_empty() {
                return format!(
                    "`{name}` bound at {module}:{}, dropped by {listed}",
                    rows.start
                );
            }
        }
        String::new()
    }

    /// The attribution and hunk one break carries, which is the rules the
    /// format run's records trace it to, or the rules reproducing it alone.
    fn explain(&self, brk: &Break) -> (String, Vec<String>) {
        let (file, row) = &brk.frame;
        let before = fs_err::read_to_string(self.stage.original.join(file)).unwrap_or_default();
        let after = fs_err::read_to_string(self.formatted.join(file)).unwrap_or_default();
        let was: Vec<_> = before.lines().collect();
        let now: Vec<_> = after.lines().collect();
        let mut clauses = Vec::new();
        if let Some(row) = *row {
            let rows = mapped_rows(&was, &now, row);
            let line = now.get(row - 1).unwrap_or(&"").trim();
            let under = self.fitting(file, |edits| reaches(edits, &rows, line));
            if !under.is_empty() {
                clauses.push(format!("under {under}"));
            }
        }
        if brk.name.is_some() {
            let clause = self.binding(brk);
            if !clause.is_empty() {
                clauses.push(clause);
            }
        }
        let attribution = if clauses.is_empty() {
            match self.alone(brk) {
                alone if alone.is_empty() => "no single rule reproduces it".to_owned(),
                alone => format!("reproduced by {alone} alone"),
            }
        } else {
            clauses.join(", ")
        };
        (
            attribution,
            hunk(&was, &now, *row, brk.name.as_deref().unwrap_or_default()),
        )
    }

    /// The rules whose fixes to one file satisfy a test, joined in pipeline
    /// order.
    fn fitting(&self, file: &str, fits: impl Fn(&[EditRows]) -> bool) -> String {
        let hit: BTreeSet<_> = self
            .fixes
            .get(file)
            .into_iter()
            .flatten()
            .filter(|(_, edits)| fits(edits))
            .map(|(rule, _)| *rule)
            .collect();
        let ordered: Vec<_> = Pipeline::known_ids()
            .iter()
            .filter(|rule| hit.contains(rule))
            .copied()
            .collect();
        render_slugs(&ordered).to_string()
    }

    /// The file and row a break points at, which is the deepest traceback
    /// frame under the formatted tree, otherwise the row binding the name it
    /// turns on, and the module alone where neither exists.
    fn locate(&self, brk: &Break) -> (String, Option<usize>) {
        let under = brk.formatted.frames.iter().rev().find_map(|(file, line)| {
            Path::new(file)
                .strip_prefix(self.formatted)
                .ok()
                .map(|relative| (relative.to_string_lossy().into_owned(), Some(*line)))
        });
        under.unwrap_or_else(|| {
            let text = fs_err::read_to_string(self.formatted.join(&brk.module)).unwrap_or_default();
            let row = brk
                .name
                .as_deref()
                .and_then(|name| binding_rows(&text).get(name).map(|rows| rows.start));
            (brk.module.clone(), row)
        })
    }
}

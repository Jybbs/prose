//! Attributing a break, meaning the frame it raises in, the rules whose
//! recorded fixes reach that frame or drop the binding it turns on, and the
//! rules reproducing it under one rule alone where no record does.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    path::Path,
    slice::from_ref,
};

use prose::{config::Config, pipeline::Pipeline, rules::render_slugs};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use similar::TextDiff;

use crate::{
    bindings::binding_rows,
    compare::divergence,
    diff::{hunk, mapped_rows},
    execute::Runner,
    fixes::{drops, reaches},
    format::format_tree,
    outcome::relative_to,
    records::{Break, EditRows, Fixes, Frame},
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
    /// The interpreter, deadline, and stage every re-run goes through.
    pub(crate) runner: &'a Runner,
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
                let pipeline = Pipeline::with_filters(self.config, from_ref(*rule), &[]);
                let tree =
                    self.runner
                        .stage
                        .overlay(&loaded, self.label, &brk.module, rule.as_str());
                format_tree(&tree, &pipeline);
                let ran = self
                    .runner
                    .run(&brk.module, &[&tree, &self.runner.stage.original]);
                divergence(&ran, &brk.original).is_some_and(|(why, _)| why == brk.reason)
            })
            .copied()
            .collect();
        render_slugs(&reproducing).to_string()
    }

    /// The clause naming where the name a break turns on was bound and the
    /// rules whose fixes dropped it, empty where no fix did.
    fn binding(&self, brk: &Break, name: &str) -> String {
        brk.loaded()
            .into_iter()
            .find_map(|module| {
                let (rows, text) = bound_at(&self.runner.stage.original, &module, name)?;
                let listed = self.fitting(&module, |edits| {
                    reaches(edits, &rows, "") && drops(edits, name, &text)
                });
                (!listed.is_empty()).then(|| {
                    format!(
                        "`{name}` bound at {module}:{}, dropped by {listed}",
                        rows.start
                    )
                })
            })
            .unwrap_or_default()
    }

    /// The attribution and hunk one break carries, which is the rules the
    /// format run's records trace it to, or the rules reproducing it alone.
    fn explain(&self, brk: &Break) -> (String, Vec<String>) {
        let Frame { file, row } = &brk.frame;
        let before =
            fs_err::read_to_string(self.runner.stage.original.join(file)).unwrap_or_default();
        let after = fs_err::read_to_string(self.formatted.join(file)).unwrap_or_default();
        let was: Vec<_> = before.lines().collect();
        let now: Vec<_> = after.lines().collect();
        let diff = TextDiff::from_slices(&was, &now);
        let mut clauses = Vec::new();
        if let Some(row) = *row {
            let rows = mapped_rows(&diff, row);
            let line = now.get(row - 1).map_or("", |line| line.trim());
            let under = self.fitting(file, |edits| reaches(edits, &rows, line));
            if !under.is_empty() {
                clauses.push(format!("under {under}"));
            }
        }
        if let Some(name) = brk.name.as_deref() {
            let clause = self.binding(brk, name);
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
            hunk(&diff, *row, brk.name.as_deref().unwrap_or_default()),
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
    fn locate(&self, brk: &Break) -> Frame {
        brk.formatted
            .frames
            .iter()
            .rev()
            .find_map(|(file, line)| {
                Some(Frame {
                    file: relative_to(file, from_ref(&self.formatted))?,
                    row: Some(*line),
                })
            })
            .unwrap_or_else(|| Frame {
                file: brk.module.clone(),
                row: brk.name.as_deref().and_then(|name| {
                    bound_at(self.formatted, &brk.module, name).map(|(rows, _)| rows.start)
                }),
            })
    }

    /// Locates every break and explains the first of each group sharing a
    /// frame and reason, the rest taking that one's attribution and hunk.
    pub(crate) fn attribute(&self, breaks: &mut [Break]) {
        let frames: Vec<_> = breaks.par_iter().map(|brk| self.locate(brk)).collect();
        for (brk, frame) in breaks.iter_mut().zip(frames) {
            brk.frame = frame;
        }
        let mut leaders: BTreeMap<(Frame, String), usize> = BTreeMap::new();
        let mut follows: Vec<Option<usize>> = Vec::with_capacity(breaks.len());
        for (at, brk) in breaks.iter().enumerate() {
            let key = (brk.frame.clone(), brk.reason.clone());
            let leader = *leaders.entry(key).or_insert(at);
            follows.push((leader != at).then_some(leader));
        }
        let ordered: Vec<_> = leaders.into_values().collect();
        let explained: BTreeMap<usize, (String, Vec<String>)> = ordered
            .par_iter()
            .map(|at| (*at, self.explain(&breaks[*at])))
            .collect();
        for (at, leader) in follows.iter().enumerate() {
            let (attribution, hunk) = &explained[&leader.unwrap_or(at)];
            breaks[at].attribution.clone_from(attribution);
            breaks[at].hunk.clone_from(hunk);
        }
    }
}

/// The rows binding `name` in one module of `tree`, beside that module's
/// text, `None` where the module does not read or does not bind it.
fn bound_at(tree: &Path, module: &str, name: &str) -> Option<(Range<usize>, String)> {
    let text = fs_err::read_to_string(tree.join(module)).ok()?;
    let rows = binding_rows(&text).get(name)?.clone();
    Some((rows, text))
}

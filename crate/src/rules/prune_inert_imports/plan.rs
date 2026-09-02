//! The per-alias decision the rule reaches over a module's imports, the
//! drops it applies and the reports a package `__init__` holds back.

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::{TextRange, TextSize};
use rustc_hash::FxHashSet;

use super::{
    PruneInertImports,
    annotations::annotation_names,
    future::annotations_are_inert,
    inventory::ImportNode,
    is_package_init,
    reexports::{Reexports, reexports_a_private_member},
};
use crate::{
    diagnostics::Diagnostic,
    primitives::{
        binding::BindingAnalysis,
        comments::noqa_names,
        imports::{Dropping, is_star},
    },
    rule::RuleId,
    rules::reflow_imports::Folds,
    source::Source,
};

/// The code `flake8` and its successors report an unread import under,
/// which a `noqa` naming it marks as deliberate.
const REEXPORT_CODE: &str = "F401";

/// The alias drops the rule applies, one entry per pruned statement,
/// beside the unreferenced bindings a package `__init__.py` holds.
#[derive(Default)]
pub(super) struct Plan<'a> {
    drops: Vec<Dropping<'a>>,
    folds: Option<&'a Folds>,
    reports: Vec<Report<'a>>,
}

impl<'a> Plan<'a> {
    /// Walks the module-scope imports of `source`, dropping every
    /// candidate and holding back the unreferenced ones a package
    /// `__init__` re-exports. A repeat the pass drops no longer rebinds
    /// the name, so the binding it repeated reads as write-once.
    pub(super) fn of(rule: &'a PruneInertImports, source: &'a Source) -> Self {
        let body = &source.ast().body;
        let nodes: Vec<(usize, ImportNode<'a>)> = body
            .iter()
            .enumerate()
            .filter_map(|(slot, stmt)| ImportNode::of(stmt).map(|node| (slot, node)))
            .collect();
        if nodes.is_empty() {
            return Self::default();
        }
        let analysis = source.binding_analysis();
        let reexports = Reexports::of(body);
        let noqa_held: FxHashSet<usize> = nodes
            .iter()
            .positions(|(slot, _)| noqa_names(source, &body[*slot], REEXPORT_CODE))
            .collect();
        let package_init = is_package_init(source);
        let annotated = if rule.unreferenced {
            annotation_names(source.ast())
        } else {
            FxHashSet::default()
        };
        let directive_is_inert = rule.unreferenced
            && nodes
                .iter()
                .any(|(_, node)| node.future_annotations().is_some())
            && annotations_are_inert(rule, source);
        let repeats = if rule.duplicates {
            repeat_writes(&nodes, &reexports, &noqa_held)
        } else {
            FxHashSet::default()
        };

        let mut dropped: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut reports = Vec::new();
        for (statement, (_, node)) in nodes.iter().enumerate() {
            if noqa_held.contains(&statement) {
                continue;
            }
            let directive = node.future_annotations();
            for (index, alias) in node.names().iter().enumerate() {
                let bound = node.bound(alias);
                let candidacy = if reexports.holds(alias, bound) {
                    None
                } else if repeats.contains(&alias.range.start()) {
                    Some(Candidacy::Inert)
                } else if !rule.unreferenced || is_star(alias) {
                    None
                } else if node.is_future() {
                    (directive_is_inert && directive == Some(index)).then_some(Candidacy::Inert)
                } else if reexports_a_private_member(node) {
                    None
                } else {
                    is_unreferenced(analysis, bound, &repeats, &annotated)
                        .then_some(Candidacy::Unreferenced)
                };
                match candidacy {
                    Some(Candidacy::Unreferenced) if package_init => reports.push(Report {
                        name: bound,
                        range: alias.range,
                    }),
                    Some(_) => dropped[statement].push(index),
                    None => {}
                }
            }
        }

        Self {
            drops: nodes
                .iter()
                .zip(dropped)
                .filter(|(_, dropped)| !dropped.is_empty())
                .map(|((slot, node), dropped)| Dropping {
                    dropped,
                    names: node.names(),
                    range: node.range(),
                    slot: *slot,
                })
                .collect(),
            folds: Some(&rule.folds),
            reports,
        }
    }

    /// One lint per unreferenced binding a package `__init__` holds.
    pub(super) fn diagnostics(&self, rule: RuleId) -> Vec<Diagnostic> {
        self.reports
            .iter()
            .map(|report| {
                Diagnostic::lint(
                    rule,
                    report.range,
                    format!(
                        "`{}` is imported and never referenced. Dropping it from a package's `__init__` changes what the package re-exports, so remove the line by hand once nothing outside this file reads it",
                        report.name,
                    ),
                )
            })
            .collect()
    }

    /// One fix group per pruned statement, a comment-led statement
    /// losing every alias landing on the import its comment heads once
    /// the later rules have laid the block out.
    pub(super) fn edits(&self, source: &Source) -> Vec<Vec<Edit>> {
        let Some(folds) = self.folds else {
            return Vec::new();
        };
        folds.prune(source, &self.drops)
    }
}

/// Why an alias is a prune candidate.
enum Candidacy {
    /// A repeat the interpreter answers out of `sys.modules` without
    /// running the module again, or a `from __future__ import
    /// annotations` directive the annotation analysis has cleared.
    Inert,
    /// A binding the module's own reference count never reaches.
    Unreferenced,
}

/// One unreferenced binding a package `__init__.py` holds.
struct Report<'a> {
    name: &'a str,
    range: TextRange,
}

/// True when nothing in the module reaches `bound`, counting neither a
/// write in `repeats` as a rebind nor a name in `annotated` as unread.
fn is_unreferenced(
    analysis: &BindingAnalysis,
    bound: &str,
    repeats: &FxHashSet<TextSize>,
    annotated: &FxHashSet<String>,
) -> bool {
    analysis.module_usage_count(bound) == 0
        && !analysis.module_reassigned_without(bound, |offset| repeats.contains(&offset))
        && !analysis.is_deleted(bound)
        && !annotated.contains(bound)
}

/// The write offset of every alias repeating a binding an earlier
/// import already made. An alias the re-export surface holds keeps its
/// binding, as does one on a statement a `noqa` comment trails, so
/// neither offset stays in.
fn repeat_writes(
    nodes: &[(usize, ImportNode<'_>)],
    reexports: &Reexports<'_>,
    noqa_held: &FxHashSet<usize>,
) -> FxHashSet<TextSize> {
    let mut bound_sources = FxHashSet::default();
    let mut repeats = FxHashSet::default();
    for (statement, (_, node)) in nodes.iter().enumerate() {
        for alias in node.names() {
            let bound = node.bound(alias);
            let unseen = bound_sources.insert((bound, node.source(alias)));
            if !unseen && !noqa_held.contains(&statement) && !reexports.holds(alias, bound) {
                repeats.insert(alias.range.start());
            }
        }
    }
    repeats
}

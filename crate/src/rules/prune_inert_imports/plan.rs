//! The per-alias decision the rule reaches over a module's imports, the
//! drops it applies and the reports a package `__init__` holds back.

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::{TextRange, TextSize};
use rustc_hash::FxHashSet;

use super::{
    PruneInertImports,
    annotations::type_expression_names,
    future::annotations_are_inert,
    inventory::ImportNode,
    is_package_init,
    reexports::{REEXPORT_CODE, Reexports, defines_no_own_name, reexports_a_private_member},
};
use crate::{
    diagnostics::Diagnostic,
    primitives::{
        binding::BindingAnalysis,
        comments::noqa_names,
        imports::{Dropping, defers_annotations, is_star},
    },
    rules::{RuleId, reflow_imports::Folds},
    source::Source,
};

/// The alias drops the rule applies, one entry per pruned statement,
/// beside the unreferenced bindings a package `__init__.py` holds.
pub(super) struct Plan<'a> {
    drops: Vec<Dropping<'a>>,
    folds: &'a Folds,
    reports: Vec<Report<'a>>,
}

impl<'a> Plan<'a> {
    /// Walks the module-scope imports of `source`, dropping each
    /// candidate, holding an unreferenced binding where the module
    /// writes no `__all__`, and reporting one a package `__init__`
    /// binds instead of dropping it. A repeat the pass drops no longer
    /// rebinds the name, so the binding it repeated reads as
    /// write-once.
    pub(super) fn of(rule: &'a PruneInertImports, source: &'a Source) -> Self {
        let body = &source.ast().body;
        let nodes: Vec<(usize, ImportNode<'a>)> = body
            .iter()
            .enumerate()
            .filter_map(|(slot, stmt)| ImportNode::of(stmt).map(|node| (slot, node)))
            .collect();
        if nodes.is_empty() {
            return Self {
                drops: Vec::new(),
                folds: &rule.folds,
                reports: Vec::new(),
            };
        }
        let analysis = source.binding_analysis();
        let reexports = Reexports::of(source);
        let noqa_held: FxHashSet<usize> = nodes
            .iter()
            .positions(|(slot, _)| noqa_names(source, &body[*slot], REEXPORT_CODE))
            .collect();
        let package_init = is_package_init(source);
        let shim = !reexports.declares_a_surface() && defines_no_own_name(analysis, body);
        let type_names = if rule.unreferenced {
            type_expression_names(source.ast())
        } else {
            FxHashSet::default()
        };
        let directive_is_inert =
            rule.unreferenced && defers_annotations(body) && annotations_are_inert(rule, source);
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
            let private_source = reexports_a_private_member(node);
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
                } else if private_source {
                    None
                } else {
                    is_unreferenced(analysis, bound, &repeats, &type_names)
                        .then_some(Candidacy::Unreferenced)
                };
                let held = if package_init {
                    Some(Held::PackageInit)
                } else {
                    shim.then_some(Held::NoSurface)
                };
                match (candidacy, held) {
                    (Some(Candidacy::Unreferenced), Some(held)) => reports.push(Report {
                        held,
                        name: bound,
                        range: alias.range,
                    }),
                    (Some(_), _) => dropped[statement].push(index),
                    (None, _) => {}
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
            folds: &rule.folds,
            reports,
        }
    }

    /// One lint per unreferenced binding the rule holds back rather than
    /// drops, each naming what about the module held it.
    pub(super) fn diagnostics(&self, rule: RuleId) -> Vec<Diagnostic> {
        self.reports
            .iter()
            .map(|report| {
                let reason = match report.held {
                    Held::NoSurface => "This module writes no `__all__` and binds no name of its own, so nothing in it distinguishes a name a sibling imports from a binding the module stopped using. List the name in `__all__` or remove the import by hand to settle which it is",
                    Held::PackageInit => "Dropping it from a package's `__init__` changes what the package re-exports, so remove the line by hand once nothing outside this file reads it",
                };
                Diagnostic::lint(
                    rule,
                    report.range,
                    format!("`{}` is imported and never referenced. {reason}", report.name),
                )
            })
            .collect()
    }

    /// One fix group per pruned statement, a comment-led statement
    /// losing every alias landing on the import its comment heads once
    /// the later rules have laid the block out.
    pub(super) fn edits(&self, source: &Source) -> Vec<Vec<Edit>> {
        self.folds.prune(source, &self.drops)
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

/// What about a module holds an unreferenced binding back from the
/// drop.
#[derive(Clone, Copy)]
enum Held {
    /// A module writing no `__all__` and binding no name of its own,
    /// which is the shape a compatibility shim takes.
    NoSurface,
    /// A package's `__init__.py` or its stub, whose bindings are the
    /// package's public API.
    PackageInit,
}

/// One unreferenced binding the rule reports rather than drops.
struct Report<'a> {
    held: Held,
    name: &'a str,
    range: TextRange,
}

/// True when nothing in the module reaches `bound`, counting neither a
/// write in `repeats` as a rebind nor a name in `type_names` as unread.
fn is_unreferenced(
    analysis: &BindingAnalysis,
    bound: &str,
    repeats: &FxHashSet<TextSize>,
    type_names: &FxHashSet<String>,
) -> bool {
    analysis.module_usage_count(bound) == 0
        && !analysis.module_reassigned_without(bound, |offset| repeats.contains(&offset))
        && !analysis.is_deleted(bound)
        && !type_names.contains(bound)
}

/// The write offset of every alias repeating a binding an earlier
/// import already made. An alias the re-export surface holds and one
/// on a statement a `noqa` comment trails are both left out.
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

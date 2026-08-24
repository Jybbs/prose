//! The per-alias decision the rule reaches over a module's imports, the
//! drops it applies and the reports a package `__init__` holds back.

use std::collections::{HashMap, HashSet};

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use super::{
    PruneInertImports,
    annotations::annotation_names,
    future::annotations_are_inert,
    inventory::{ImportNode, is_star},
    is_package_init,
    reexports::Reexports,
};
use crate::{
    diagnostics::Diagnostic, primitives::imports::Dropping, rule::RuleId,
    rules::reflow_imports::Folds, source::Source,
};

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
        let package_init = is_package_init(source);
        let annotated = if rule.unreferenced {
            annotation_names(source.ast())
        } else {
            HashSet::new()
        };
        let directive_is_inert = rule.unreferenced
            && nodes
                .iter()
                .any(|(_, node)| node.future_annotations().is_some())
            && annotations_are_inert(source, rule.target_version);

        let mut bound_sources = HashSet::new();
        let mut repeats: HashMap<&str, usize> = HashMap::new();
        let repeated: Vec<Vec<bool>> = nodes
            .iter()
            .map(|(_, node)| {
                node.names()
                    .iter()
                    .map(|alias| {
                        let bound = node.bound(alias);
                        let repeat =
                            rule.duplicates && !bound_sources.insert((bound, node.source(alias)));
                        if repeat && !reexports.holds(alias, bound) {
                            *repeats.entry(bound).or_default() += 1;
                        }
                        repeat
                    })
                    .collect()
            })
            .collect();
        let mut dropped: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut reports = Vec::new();
        for (statement, (_, node)) in nodes.iter().enumerate() {
            let directive = node.future_annotations();
            for (index, alias) in node.names().iter().enumerate() {
                let bound = node.bound(alias);
                let candidacy = if reexports.holds(alias, bound) {
                    None
                } else if repeated[statement][index] {
                    Some(Candidacy::Inert)
                } else if !rule.unreferenced || is_star(alias) {
                    None
                } else if node.is_future() {
                    (directive_is_inert && directive == Some(index)).then_some(Candidacy::Inert)
                } else {
                    let repeats = repeats.get(bound).copied().unwrap_or_default();
                    (analysis.module_usage_count(bound) == 0
                        && !analysis.module_reassigned_beyond(bound, repeats)
                        && !analysis.is_deleted(bound)
                        && !annotated.contains(bound))
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

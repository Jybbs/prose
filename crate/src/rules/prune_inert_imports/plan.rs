//! The per-alias decision the rule reaches over a module's imports, the
//! drops it applies and the reports a package `__init__` holds back.

use std::collections::HashSet;

use ruff_diagnostics::Edit;
use ruff_python_ast::Alias;
use ruff_text_size::{TextRange, TextSize};

use super::{
    PruneInertImports,
    annotations::annotation_names,
    future::annotations_are_inert,
    inventory::{ImportNode, is_star},
    is_package_init,
    reexports::Reexports,
};
use crate::{
    diagnostics::Diagnostic,
    primitives::{binding::BindingAnalysis, imports::prune_import_aliases},
    rule::RuleId,
    source::Source,
};

/// The alias drops the rule applies, one entry per pruned statement,
/// beside the unreferenced bindings a package `__init__.py` holds.
#[derive(Default)]
pub(super) struct Plan<'a> {
    drops: Vec<Pruned<'a>>,
    reports: Vec<Report<'a>>,
}

impl<'a> Plan<'a> {
    /// Walks the module-scope imports of `source`, dropping every
    /// candidate and holding back the unreferenced ones a package
    /// `__init__` re-exports.
    pub(super) fn of(rule: &PruneInertImports, source: &'a Source) -> Self {
        let body = &source.ast().body;
        let nodes: Vec<ImportNode<'a>> = body.iter().filter_map(ImportNode::of).collect();
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
            && nodes.iter().any(|node| node.future_annotations().is_some())
            && annotations_are_inert(source, rule.target_version);
        let repeats = if rule.duplicates {
            repeat_writes(&nodes, &reexports)
        } else {
            HashSet::new()
        };

        let mut dropped: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut reports = Vec::new();
        for (statement, node) in nodes.iter().enumerate() {
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
                .map(|(node, dropped)| Pruned {
                    dropped,
                    names: node.names(),
                    range: node.range(),
                })
                .collect(),
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

    /// One fix group per pruned statement.
    pub(super) fn edits(&self, source: &Source) -> Vec<Vec<Edit>> {
        self.drops
            .iter()
            .map(|pruned| {
                prune_import_aliases(source, pruned.range, pruned.names, |index| {
                    !pruned.dropped.contains(&index)
                })
            })
            .filter(|edits| !edits.is_empty())
            .collect()
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

/// The alias positions one import statement loses.
struct Pruned<'a> {
    dropped: Vec<usize>,
    names: &'a [Alias],
    range: TextRange,
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
    repeats: &HashSet<TextSize>,
    annotated: &HashSet<String>,
) -> bool {
    analysis.module_usage_count(bound) == 0
        && !analysis.module_reassigned_without(bound, |offset| repeats.contains(&offset))
        && !analysis.is_deleted(bound)
        && !annotated.contains(bound)
}

/// The write offset of every alias repeating a binding an earlier
/// import already made. An alias the re-export surface holds keeps its
/// binding, so its offset stays out.
fn repeat_writes(nodes: &[ImportNode<'_>], reexports: &Reexports<'_>) -> HashSet<TextSize> {
    let mut bound_sources = HashSet::new();
    let mut repeats = HashSet::new();
    for node in nodes {
        for alias in node.names() {
            let bound = node.bound(alias);
            let unseen = bound_sources.insert((bound, node.source(alias)));
            if !unseen && !reexports.holds(alias, bound) {
                repeats.insert(alias.range.start());
            }
        }
    }
    repeats
}

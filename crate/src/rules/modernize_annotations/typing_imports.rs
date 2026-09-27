//! Indexes the module's `typing` imports for the rewrite to resolve
//! against, and drops each alias the rewrite leaves unread.

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{Alias, Stmt, name::QualifiedName};
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use crate::{
    primitives::{
        binding::{
            bare_import_bound_name, bare_import_path, from_import_bound_name, top_level_module,
        },
        imports::Dropping,
    },
    rules::{modernize_annotations::ModernizeAnnotations, reflow_imports::Folds},
    source::Source,
};

/// The module's `typing` and `typing_extensions` imports, indexing each
/// bound name to the qualified path it names and holding the statements
/// that bound them.
pub(super) struct TypingImports<'a> {
    aliases: FxHashMap<&'a str, QualifiedName<'a>>,
    statements: Vec<TypingImport<'a>>,
}

impl<'a> TypingImports<'a> {
    /// Reads the module's top-level imports, `None` when none of them
    /// binds a `typing` name. An import below module scope and a
    /// relative `from .typing import …` are both skipped.
    pub(super) fn collect(body: &'a [Stmt]) -> Option<Self> {
        let mut aliases = FxHashMap::default();
        let mut statements = Vec::new();
        for (slot, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Import(node) => {
                    let mut bound = node
                        .names
                        .iter()
                        .filter(|alias| is_typing_root(top_level_module(alias.name.as_str())))
                        .map(|alias| {
                            (
                                bare_import_bound_name(alias),
                                QualifiedName::user_defined(bare_import_path(alias)),
                            )
                        })
                        .peekable();
                    if bound.peek().is_none() {
                        continue;
                    }
                    aliases.extend(bound);
                    statements.push(TypingImport {
                        bare: true,
                        names: &node.names,
                        range: node.range,
                        slot,
                    });
                }
                Stmt::ImportFrom(node) if node.level == 0 => {
                    let Some(module) = node
                        .module
                        .as_ref()
                        .filter(|module| is_typing_root(module.as_str()))
                    else {
                        continue;
                    };
                    aliases.extend(node.names.iter().map(|alias| {
                        let path = QualifiedName::user_defined(module.as_str());
                        (
                            from_import_bound_name(alias),
                            path.append_member(alias.name.as_str()),
                        )
                    }));
                    statements.push(TypingImport {
                        bare: false,
                        names: &node.names,
                        range: node.range,
                        slot,
                    });
                }
                _ => {}
            }
        }
        (!aliases.is_empty()).then_some(Self {
            aliases,
            statements,
        })
    }

    pub(super) fn aliases(&self) -> &FxHashMap<&'a str, QualifiedName<'a>> {
        &self.aliases
    }

    /// One fix group per import statement, dropping every alias whose
    /// reads the rewrite consumed entirely and keeping one with a
    /// surviving reference. A comment-led statement losing every alias
    /// lands on the import `folds` moves its comment to.
    pub(super) fn prune(
        &self,
        source: &Source,
        consumed: &FxHashMap<&str, usize>,
        folds: &Folds,
    ) -> Vec<Vec<Edit>> {
        let analysis = source.binding_analysis();
        let unread = |bound: &str| {
            consumed
                .get(bound)
                .is_some_and(|&rewritten| rewritten == analysis.module_usage_count(bound))
        };
        let drops: Vec<Dropping> = self
            .statements
            .iter()
            .map(|import| Dropping {
                dropped: import
                    .names
                    .iter()
                    .positions(|alias| import.orphaned(alias, &unread))
                    .collect(),
                names: import.names,
                range: import.range,
                slot: import.slot,
            })
            .filter(|drop| !drop.dropped.is_empty())
            .collect();
        folds.prune(source, &drops, ModernizeAnnotations::SLUG)
    }
}

/// One collected `typing` import statement, `bare` telling the
/// `import typing` form from the `from typing import …` one.
struct TypingImport<'a> {
    bare: bool,
    names: &'a [Alias],
    range: TextRange,
    slot: usize,
}

impl TypingImport<'_> {
    /// True when `alias` binds a name the rewrite read out entirely, an
    /// unaliased dotted `import a.b` holding regardless.
    fn orphaned(&self, alias: &Alias, unread: &impl Fn(&str) -> bool) -> bool {
        if !self.bare {
            return unread(from_import_bound_name(alias));
        }
        bare_import_path(alias) == alias.name.as_str() && unread(bare_import_bound_name(alias))
    }
}

/// True for the two module names that carry the `typing` members this
/// rule rewrites.
pub(super) fn is_typing_root(module: &str) -> bool {
    matches!(module, "typing" | "typing_extensions")
}

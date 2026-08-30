//! Indexes the module's `typing` imports for the rewrite to resolve
//! against, and drops each alias the rewrite leaves unread.

use ruff_diagnostics::Edit;
use ruff_python_ast::{Alias, Stmt, name::QualifiedName};
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use crate::{
    primitives::{
        binding::{bare_import_bound_name, from_import_bound_name, top_level_module},
        imports::Dropping,
    },
    rules::reflow_imports::Folds,
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
    /// relative `from .typing import …` both go unread.
    pub(super) fn collect(body: &'a [Stmt]) -> Option<Self> {
        let mut aliases = FxHashMap::default();
        let mut statements = Vec::new();
        for (slot, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Import(node) => {
                    let mut bound = node
                        .names
                        .iter()
                        .filter_map(|alias| {
                            let name = alias.name.as_str();
                            let bound = bare_import_bound_name(alias);
                            is_typing_root(top_level_module(name)).then(|| {
                                let path = if alias.asname.is_some() { name } else { bound };
                                (bound, QualifiedName::user_defined(path))
                            })
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
    /// bound name `consumed` read as many times as the module reads it
    /// at all and leaving an alias with a surviving reference in place.
    /// A comment-led statement losing every alias lands on the import
    /// `folds` carries its comment onto.
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
                dropped: (0..import.names.len())
                    .filter(|&index| import.orphaned(&import.names[index], &unread))
                    .collect(),
                names: import.names,
                range: import.range,
                slot: import.slot,
            })
            .collect();
        folds.prune(source, &drops)
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
    /// True when `alias` binds a name the rewrite read out entirely. An
    /// unaliased `import a.b` holds, in that dropping it would unbind
    /// `a.b` as well as the `a` the rewrite read.
    fn orphaned(&self, alias: &Alias, unread: &impl Fn(&str) -> bool) -> bool {
        if !self.bare {
            return unread(from_import_bound_name(alias));
        }
        (alias.asname.is_some() || !alias.name.contains('.'))
            && unread(bare_import_bound_name(alias))
    }
}

/// True for the two module names that carry the `typing` members this
/// rule rewrites.
fn is_typing_root(module: &str) -> bool {
    matches!(module, "typing" | "typing_extensions")
}

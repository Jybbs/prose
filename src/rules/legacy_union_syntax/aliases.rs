//! Resolution of `typing` / `typing_extensions` import aliases
//! into the qualified names the union-syntax rule rewrites.

use std::collections::HashMap;

use ruff_python_ast::{Identifier, Stmt, name::QualifiedName};

use crate::primitives::binding::{from_import_bound_name, top_level_module};

/// Walks every top-level `Stmt::Import` and `Stmt::ImportFrom`,
/// recording the bound name and qualified path for each alias whose
/// source is `typing` or `typing_extensions`. Imports nested below
/// module scope are skipped.
pub(super) fn collect_typing_aliases(body: &[Stmt]) -> HashMap<&str, QualifiedName<'_>> {
    let mut imports = HashMap::new();
    for stmt in body {
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    let name = alias.name.as_str();
                    if !is_typing_root(top_level_module(name)) {
                        continue;
                    }
                    let bound = alias.asname.as_ref().map_or(name, Identifier::as_str);
                    imports.insert(bound, QualifiedName::user_defined(name));
                }
            }
            Stmt::ImportFrom(import) => {
                let Some(module) = import
                    .module
                    .as_ref()
                    .filter(|m| is_typing_root(m.as_str()))
                else {
                    continue;
                };
                for alias in &import.names {
                    let bound = from_import_bound_name(alias);
                    imports.insert(
                        bound,
                        QualifiedName::user_defined(module.as_str())
                            .append_member(alias.name.as_str()),
                    );
                }
            }
            _ => {}
        }
    }
    imports
}

fn is_typing_root(module: &str) -> bool {
    matches!(module, "typing" | "typing_extensions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    #[test]
    fn collects_aliased_from_import() {
        let source = parse("from typing import Optional as Opt\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("Opt").map(QualifiedName::segments),
            Some(["typing", "Optional"].as_slice()),
        );
    }

    #[test]
    fn collects_aliased_module_import() {
        let source = parse("import typing as t\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("t").map(QualifiedName::segments),
            Some(["typing"].as_slice()),
        );
    }

    #[test]
    fn collects_typing_extensions_alias() {
        let source = parse("from typing_extensions import Union\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("Union").map(QualifiedName::segments),
            Some(["typing_extensions", "Union"].as_slice()),
        );
    }

    #[test]
    fn ignores_non_typing_imports() {
        let source = parse("from collections import OrderedDict\n");
        assert!(collect_typing_aliases(&source.ast().body).is_empty());
    }

    #[test]
    fn rejects_non_typing_bare_import() {
        let source = parse("import os\n");
        assert!(collect_typing_aliases(&source.ast().body).is_empty());
    }
}

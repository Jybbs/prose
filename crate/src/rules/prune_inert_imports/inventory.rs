//! Wraps a module-scope import statement of either form, so an alias
//! reads back the name it binds and the qualified path it names at its
//! source.

use ruff_python_ast::{
    Alias, Stmt, StmtImport, StmtImportFrom, helpers::collect_import_from_member,
    name::QualifiedName,
};
use ruff_text_size::TextRange;

use crate::primitives::{
    binding::{bare_import_bound_name, from_import_bound_name},
    imports::{future_annotations_alias, is_future},
};

/// One module-scope `import` or `from`-import statement.
pub(super) enum ImportNode<'a> {
    Bare(&'a StmtImport),
    From(&'a StmtImportFrom),
}

impl<'a> ImportNode<'a> {
    /// Wraps `stmt` when it imports, `None` for every other statement.
    pub(super) fn of(stmt: &'a Stmt) -> Option<Self> {
        match stmt {
            Stmt::Import(node) => Some(Self::Bare(node)),
            Stmt::ImportFrom(node) => Some(Self::From(node)),
            _ => None,
        }
    }

    /// The name `alias` binds.
    pub(super) fn bound(&self, alias: &'a Alias) -> &'a str {
        match self {
            Self::Bare(_) => bare_import_bound_name(alias),
            Self::From(_) => from_import_bound_name(alias),
        }
    }

    /// The position of the `annotations` alias in a
    /// `from __future__ import …`, `None` for every other import.
    pub(super) fn future_annotations(&self) -> Option<usize> {
        match self {
            Self::Bare(_) => None,
            Self::From(node) => future_annotations_alias(node),
        }
    }

    /// True for an absolute `from __future__ import …`.
    pub(super) fn is_future(&self) -> bool {
        matches!(self, Self::From(node) if is_future(node))
    }

    /// The module a `from`-import takes its names out of, `None` for a
    /// bare import and for the dot-only relative form.
    pub(super) fn module(&self) -> Option<&'a str> {
        match self {
            Self::Bare(_) => None,
            Self::From(node) => node.module.as_deref(),
        }
    }

    pub(super) fn names(&self) -> &'a [Alias] {
        match self {
            Self::Bare(node) => &node.names,
            Self::From(node) => &node.names,
        }
    }

    pub(super) fn range(&self) -> TextRange {
        match self {
            Self::Bare(node) => node.range,
            Self::From(node) => node.range,
        }
    }

    /// The qualified path `alias` names at its source, which an exact
    /// repeat matches on alongside the bound name.
    pub(super) fn source(&self, alias: &'a Alias) -> QualifiedName<'a> {
        match self {
            Self::Bare(_) => QualifiedName::from_dotted_name(alias.name.as_str()),
            Self::From(node) => {
                collect_import_from_member(node.level, node.module.as_deref(), alias.name.as_str())
            }
        }
    }
}

/// True when `alias` marks a PEP 484 explicit re-export, binding a name
/// back to itself.
pub(super) fn is_self_alias(alias: &Alias) -> bool {
    alias
        .asname
        .as_ref()
        .is_some_and(|asname| asname.as_str() == alias.name.as_str())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{primitives::imports::is_star, source::Source, testing::parse};

    fn first_alias(source: &Source) -> (ImportNode<'_>, &Alias) {
        let node = ImportNode::of(&source.ast().body[0]).expect("an import statement");
        let alias = &node.names()[0];
        (node, alias)
    }

    #[rstest]
    #[case("import os.path\n", "os")]
    #[case("import os.path as p\n", "p")]
    #[case("from typing import Any\n", "Any")]
    #[case("from typing import Any as A\n", "A")]
    fn bound_reads_the_name_each_form_binds(#[case] src: &str, #[case] expected: &str) {
        let source = parse(src);
        let (node, alias) = first_alias(&source);
        assert_eq!(node.bound(alias), expected);
    }

    #[rstest]
    #[case("from json import loads as loads\n", true)]
    #[case("from json import loads as l\n", false)]
    #[case("from json import loads\n", false)]
    fn is_self_alias_matches_the_redundant_form(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let (_, alias) = first_alias(&source);
        assert_eq!(is_self_alias(alias), expected);
    }

    #[rstest]
    #[case("from os.path import *\n", true)]
    #[case("from os.path import join\n", false)]
    fn is_star_matches_only_the_star_member(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let (_, alias) = first_alias(&source);
        assert_eq!(is_star(alias), expected);
    }

    #[rstest]
    #[case("import os.path\n", &["os", "path"])]
    #[case("from typing import Any\n", &["typing", "Any"])]
    #[case("from . import shared\n", &[".", "shared"])]
    #[case("from ..pkg import shared\n", &[".", ".", "pkg", "shared"])]
    fn source_spells_the_qualified_path_the_alias_names(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let source = parse(src);
        let (node, alias) = first_alias(&source);
        assert_eq!(node.source(alias).segments(), expected);
    }
}

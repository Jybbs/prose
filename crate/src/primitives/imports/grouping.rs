//! The canonical section an import belongs to and the key its run
//! sorts by.

use std::cmp::Reverse;

use ruff_python_ast::{Alias, Stmt, StmtImportFrom};

use super::*;

/// What distinguishes one `from`-import's module from another, the
/// leading-dot count alongside the module name.
pub(crate) type ModuleKey<'a> = (u32, Option<&'a str>);

/// Canonical import group. Derived `Ord` ranks the variants in
/// declaration order, so a sort by group lands `__future__` imports
/// first, bare imports next, then external `from` imports, and
/// local-package imports last.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ImportGroup {
    Future,
    Bare,
    ExternalFrom,
    Local,
}

/// Returns the canonical group of an `import` or `from`-import
/// statement, `None` for any other. An absolute `from __future__` is
/// its own group. A `from` import is local when relative or first-party
/// by root package, a bare import when any aliased root is first-party.
pub(crate) fn import_group(stmt: &Stmt, first_party: &[String]) -> Option<ImportGroup> {
    let (local, external) = match stmt {
        Stmt::Import(i) => (
            i.names
                .iter()
                .any(|a| is_first_party(a.name.as_str(), first_party)),
            ImportGroup::Bare,
        ),
        Stmt::ImportFrom(i) if is_future(i) => return Some(ImportGroup::Future),
        Stmt::ImportFrom(i) => (
            i.level > 0
                || i.module
                    .as_deref()
                    .is_some_and(|m| is_first_party(m, first_party)),
            ImportGroup::ExternalFrom,
        ),
        _ => return None,
    };
    Some(if local { ImportGroup::Local } else { external })
}

/// Composite import sort key, the canonical group order ahead of a
/// per-kind sort, bare before `from`, bare by least alias name and
/// `from` by `(relative, depth, module)`. An absolute `from` import
/// leads every relative one, and relative imports run furthest to
/// closest, so `..pkg` precedes `.pkg`. Ungrouped, every group below
/// `__future__` collapses to one rank. `None` pins a non-import.
pub(crate) fn import_sort_key<'a>(
    stmt: &'a Stmt,
    first_party: &[String],
    grouped: bool,
) -> Option<(ImportGroup, u8, bool, Reverse<u32>, &'a str)> {
    let group = import_group(stmt, first_party)?;
    let rank = if grouped {
        group
    } else {
        group.min(ImportGroup::Bare)
    };
    Some(match stmt {
        Stmt::Import(i) => (rank, 0, false, Reverse(0), least_alias(&i.names)),
        Stmt::ImportFrom(i) => (
            rank,
            1,
            i.level > 0,
            Reverse(i.level),
            i.module.as_deref().unwrap_or_default(),
        ),
        _ => unreachable!("import_group returns Some only for import statements"),
    })
}

/// The module a `from`-import reads, its leading-dot count beside the
/// module name, what tells one such import's module from another's.
pub(crate) fn module_key(node: &StmtImportFrom) -> ModuleKey<'_> {
    (node.level, node.module.as_deref())
}

/// True when the root package of `name` (the substring up to the
/// first `.`) appears in `first_party`.
fn is_first_party(name: &str, first_party: &[String]) -> bool {
    let root = name.split_once('.').map_or(name, |(root, _)| root);
    first_party.iter().any(|p| p == root)
}

/// Returns the alphabetically least alias name in a bare import's
/// name list. An `import` statement always binds at least one name.
fn least_alias(names: &[Alias]) -> &str {
    names
        .iter()
        .map(|a| a.name.as_str())
        .min()
        .expect("import binds at least one name")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case("import os\n", &[], Some(ImportGroup::Bare))]
    #[case("import myapp\n", &["myapp"], Some(ImportGroup::Local))]
    #[case("import myapp.core\n", &["myapp"], Some(ImportGroup::Local))]
    #[case("import os, myapp\n", &["myapp"], Some(ImportGroup::Local))]
    #[case("import myapplication\n", &["myapp"], Some(ImportGroup::Bare))]
    #[case("from collections import Counter\n", &[], Some(ImportGroup::ExternalFrom))]
    #[case("from myapp import app\n", &["myapp"], Some(ImportGroup::Local))]
    #[case("from myapp.db import Session\n", &["myapp"], Some(ImportGroup::Local))]
    #[case("from myapp import app\n", &["other"], Some(ImportGroup::ExternalFrom))]
    #[case("from . import shared\n", &[], Some(ImportGroup::Local))]
    #[case("from .sub import helpers\n", &[], Some(ImportGroup::Local))]
    #[case("from ..pkg import base\n", &[], Some(ImportGroup::Local))]
    #[case("from __future__ import annotations\n", &[], Some(ImportGroup::Future))]
    #[case("from __future__ import division\n", &["__future__"], Some(ImportGroup::Future))]
    #[case("from .__future__ import annotations\n", &[], Some(ImportGroup::Local))]
    #[case("import __future__\n", &[], Some(ImportGroup::Bare))]
    #[case("x = 1\n", &[], None)]
    fn import_group_classifies_by_kind_relativity_and_first_party(
        #[case] src: &str,
        #[case] first_party: &[&str],
        #[case] expected: Option<ImportGroup>,
    ) {
        let list: Vec<String> = first_party.iter().map(|&s| s.to_owned()).collect();
        let source = parse(src);
        assert_eq!(import_group(&source.ast().body[0], &list), expected);
    }

    #[test]
    fn import_group_ranks_future_before_bare_before_external_before_local() {
        assert!(ImportGroup::Future < ImportGroup::Bare);
        assert!(ImportGroup::Bare < ImportGroup::ExternalFrom);
        assert!(ImportGroup::ExternalFrom < ImportGroup::Local);
    }

    #[rstest]
    fn import_sort_key_pins_the_future_import_ahead_of_every_group(
        #[values(false, true)] grouped: bool,
    ) {
        let first_party = vec!["myapp".to_owned()];
        let s = parse("import myapp\nimport os\nfrom __future__ import annotations\n");
        let key = |stmt| import_sort_key(stmt, &first_party, grouped).expect("import");
        let body = &s.ast().body;
        assert!(key(&body[2]) < key(&body[0]));
        assert!(key(&body[2]) < key(&body[1]));
    }

    #[test]
    fn import_sort_key_ranks_an_absolute_from_import_ahead_of_every_relative_one() {
        let first_party = vec!["myapp".to_owned()];
        let s = parse("from .pkg import a\nfrom myapp.db import b\n");
        let key = |stmt| import_sort_key(stmt, &first_party, true).expect("import statement");
        let body = &s.ast().body;
        assert!(key(&body[1]) < key(&body[0]));
    }

    #[test]
    fn import_sort_key_ranks_groups_then_bare_before_from_within_local() {
        let first_party = vec!["myapp".to_owned()];
        let s = parse("import os\nfrom os import path\nimport myapp.core\nfrom myapp import app\n");
        let keys: Vec<_> = s
            .ast()
            .body
            .iter()
            .map(|stmt| import_sort_key(stmt, &first_party, true).expect("import statement"))
            .collect();
        assert!(
            keys[0] < keys[1] && keys[1] < keys[2] && keys[2] < keys[3],
            "expected bare-external < external-from < local-bare < local-from",
        );
    }

    #[test]
    fn import_sort_key_returns_none_for_non_import() {
        let s = parse("x = 1\n");
        assert!(import_sort_key(&s.ast().body[0], &[], true).is_none());
    }

    #[test]
    fn import_sort_key_runs_relative_imports_furthest_to_closest() {
        let s = parse("from .pkg import a\nfrom ..pkg import b\nfrom ...pkg import c\n");
        let keys: Vec<_> = s
            .ast()
            .body
            .iter()
            .map(|stmt| import_sort_key(stmt, &[], true).expect("import statement"))
            .collect();
        assert!(
            keys[2] < keys[1] && keys[1] < keys[0],
            "expected `...pkg` < `..pkg` < `.pkg`, furthest to closest",
        );
    }

    #[test]
    fn import_sort_key_ungrouped_collapses_every_group_below_the_pin() {
        let first_party = vec!["myapp".to_owned()];
        let s = parse("import myapp\nfrom collections import Counter\n");
        let key = |stmt, grouped| import_sort_key(stmt, &first_party, grouped).expect("import");
        let body = &s.ast().body;
        // Grouped: local `import myapp` sorts after external `from collections`.
        assert!(key(&body[0], true) > key(&body[1], true));
        // Ungrouped: the bare `import` leads by kind, its group ignored.
        assert!(key(&body[0], false) < key(&body[1], false));
    }

    #[test]
    fn least_alias_returns_alphabetically_min_name() {
        let s = parse("import sys, os, abc\n");
        let import = s.ast().body[0].as_import_stmt().expect("import");
        assert_eq!(least_alias(&import.names), "abc");
    }
}

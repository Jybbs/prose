//! Classifies import statements into the canonical group order
//! bare → external `from` → local-package, finds the runs of adjacent
//! imports the ordering rules act on, builds the composite sort key
//! ordering a run within and across those groups, and counts the
//! canonical blank lines dividing two imports. First-party detection
//! reads the package-name list from `[tool.prose.imports]`.

use std::ops::Range;

use ruff_python_ast::{Alias, Stmt, StmtImportFrom};

use crate::primitives::orderer::chunk_runs;

const FUTURE_ANNOTATIONS: &str = "annotations";
const FUTURE_MODULE: &str = "__future__";

/// Canonical import group. Derived `Ord` ranks the variants in
/// declaration order, so a sort by group lands bare imports first,
/// external `from` imports next, and local-package imports last.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ImportGroup {
    Bare,
    ExternalFrom,
    Local,
}

/// Returns the position of the `annotations` alias in a
/// `from __future__ import …` statement, or `None` for any other
/// import.
pub(crate) fn future_annotations_alias(node: &StmtImportFrom) -> Option<usize> {
    if node.level != 0 || node.module.as_deref() != Some(FUTURE_MODULE) {
        return None;
    }
    node.names
        .iter()
        .position(|alias| alias.name.id == FUTURE_ANNOTATIONS)
}

/// Canonical blank-line count between two adjacent import statements,
/// the one decider the import collapse, the banded import arm, and
/// `blank-lines` share. `Some(1)` divides distinct groups while
/// `grouped`, `Some(0)` seats every other import pair tight, and `None`
/// pins any pair that is not two imports. Ungrouped, the imports read as
/// one flat block, so no pair carries a divider.
pub(crate) fn import_blank_lines(
    a: &Stmt,
    b: &Stmt,
    first_party: &[String],
    grouped: bool,
) -> Option<u32> {
    let a_group = import_group(a, first_party)?;
    let b_group = import_group(b, first_party)?;
    Some(u32::from(grouped && a_group != b_group))
}

/// Returns the canonical group of an `import` or `from`-import
/// statement, or `None` for any other statement. A `from` import is
/// local when it is relative (`level > 0`) or its module's root
/// package is first-party. A bare import is local when any aliased
/// root package is first-party.
pub(crate) fn import_group(stmt: &Stmt, first_party: &[String]) -> Option<ImportGroup> {
    let (local, external) = match stmt {
        Stmt::Import(i) => (
            i.names
                .iter()
                .any(|a| is_first_party(a.name.as_str(), first_party)),
            ImportGroup::Bare,
        ),
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

/// Slot ranges of each run of two or more adjacent imports in `stmts`,
/// the unit `group-imports` partitions and `alphabetize` sorts.
pub(crate) fn import_runs(stmts: &[Stmt]) -> Vec<Range<usize>> {
    chunk_runs(stmts, |a, b| is_import(a) && is_import(b))
}

/// Composite import sort key. With `grouped`, the canonical group order
/// (bare → external `from` → local-package) leads a per-kind inner sort,
/// where bare imports sort before `from` imports, bare by least alias name
/// and `from` by `(level, module)`. Without it, the group drops so a run
/// sorts as one flat block. `None` pins any non-import statement in place.
pub(crate) fn import_sort_key<'a>(
    stmt: &'a Stmt,
    first_party: &[String],
    grouped: bool,
) -> Option<(Option<ImportGroup>, u8, u32, &'a str)> {
    let group = grouped.then_some(import_group(stmt, first_party)?);
    Some(match stmt {
        Stmt::Import(i) => (group, 0, 0, least_alias(&i.names)),
        Stmt::ImportFrom(i) => (group, 1, i.level, i.module.as_deref().unwrap_or_default()),
        _ => unreachable!("import_group returns Some only for import statements"),
    })
}

/// True when the root package of `name` (the substring up to the
/// first `.`) appears in `first_party`.
fn is_first_party(name: &str, first_party: &[String]) -> bool {
    let root = name.split_once('.').map_or(name, |(root, _)| root);
    first_party.iter().any(|p| p == root)
}

/// True for an `import` or `from`-import statement.
fn is_import(stmt: &Stmt) -> bool {
    stmt.is_import_stmt() || stmt.is_import_from_stmt()
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
    #[case("import os\nimport sys\n", true, Some(0))]
    #[case("import os\nfrom collections import deque\n", true, Some(1))]
    #[case("import os\nfrom collections import deque\n", false, Some(0))]
    #[case("import os\nimport sys\n", false, Some(0))]
    #[case("import os\nx = 1\n", true, None)]
    #[case("x = 1\nimport os\n", true, None)]
    fn import_blank_lines_scores_only_import_pairs(
        #[case] src: &str,
        #[case] grouped: bool,
        #[case] expected: Option<u32>,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(
            import_blank_lines(&body[0], &body[1], &[], grouped),
            expected
        );
    }

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
    fn import_group_ranks_bare_before_external_before_local() {
        assert!(ImportGroup::Bare < ImportGroup::ExternalFrom);
        assert!(ImportGroup::ExternalFrom < ImportGroup::Local);
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
    fn import_sort_key_ungrouped_drops_the_group_dimension() {
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

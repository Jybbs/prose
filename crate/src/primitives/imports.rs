//! Classifies import statements into the canonical group order
//! `__future__` → bare → external `from` → local-package, finds the runs
//! of adjacent imports the ordering rules act on, builds the composite
//! sort key ordering a run within and across those groups, counts the
//! canonical blank lines dividing two imports, and shapes the deletions
//! that drop the aliases a rule has left unread. First-party detection
//! reads the package-name list from `[tool.prose.imports]`.

use std::ops::Range;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Alias, Stmt, StmtImportFrom};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        comments::leading_comment_block, edit::whole_line_deletion, orderer::runs_where,
        sections::Sections,
    },
    source::Source,
};

const FUTURE_ANNOTATIONS: &str = "annotations";
const FUTURE_MODULE: &str = "__future__";

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

/// True when the module carries `from __future__ import annotations`,
/// deferring every annotation's evaluation per PEP 563.
pub(crate) fn defers_annotations(body: &[Stmt]) -> bool {
    body.iter()
        .filter_map(Stmt::as_import_from_stmt)
        .any(|node| future_annotations_alias(node).is_some())
}

/// Returns the position of the `annotations` alias in a
/// `from __future__ import …` statement, or `None` for any other
/// import.
pub(crate) fn future_annotations_alias(node: &StmtImportFrom) -> Option<usize> {
    if !is_future(node) {
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
/// `from` by `(level, module)`. Ungrouped, every group below
/// `__future__` collapses to one rank. `None` pins a non-import.
pub(crate) fn import_sort_key<'a>(
    stmt: &'a Stmt,
    first_party: &[String],
    grouped: bool,
) -> Option<(ImportGroup, u8, u32, &'a str)> {
    let group = import_group(stmt, first_party)?;
    let rank = if grouped {
        group
    } else {
        group.min(ImportGroup::Bare)
    };
    Some(match stmt {
        Stmt::Import(i) => (rank, 0, 0, least_alias(&i.names)),
        Stmt::ImportFrom(i) => (rank, 1, i.level, i.module.as_deref().unwrap_or_default()),
        _ => unreachable!("import_group returns Some only for import statements"),
    })
}

/// True for an absolute `from __future__ import …` statement.
pub(crate) fn is_future(node: &StmtImportFrom) -> bool {
    node.level == 0 && node.module.as_deref() == Some(FUTURE_MODULE)
}

/// True for an `import` or `from`-import statement.
pub(crate) fn is_import(stmt: &Stmt) -> bool {
    stmt.is_import_stmt() || stmt.is_import_from_stmt()
}

/// The deletions dropping every alias of an import statement that
/// `keep` rejects, empty when every alias survives, when the statement
/// shares its lines with other code, or when a comment sits inside it.
///
/// A statement losing all of its aliases goes whole, taking its full
/// lines. One losing a subset keeps the survivors byte-for-byte, each
/// deletion covering one run of dropped aliases together with the
/// separator binding it to the survivor beside it.
pub(crate) fn prune_import_aliases(
    source: &Source,
    stmt: TextRange,
    names: &[Alias],
    keep: impl Fn(usize) -> bool,
) -> Vec<Edit> {
    let kept: Vec<usize> = (0..names.len()).filter(|&index| keep(index)).collect();
    if kept.len() == names.len() || !stands_alone(source, stmt) || source.intersects_comment(stmt) {
        return Vec::new();
    }
    let Some(&last_kept) = kept.last() else {
        return if comment_leads(source, stmt) {
            Vec::new()
        } else {
            vec![whole_line_deletion(source, stmt)]
        };
    };
    let starts = std::iter::once(0).chain(kept.iter().map(|&index| index + 1));
    let ends = kept.iter().copied().chain(std::iter::once(names.len()));
    starts
        .zip(ends)
        .filter(|&(start, end)| start < end)
        .map(|(start, end)| {
            Edit::range_deletion(match names.get(end) {
                Some(survivor) => TextRange::new(names[start].start(), survivor.start()),
                None => TextRange::new(names[last_kept].end(), names[end - 1].end()),
            })
        })
        .collect()
}

/// Slot ranges of every import run across a sectioned body, each run
/// offset to absolute slot indices so it never spans a section divider.
/// The unit `group-imports` partitions and `alphabetize` sorts, one run
/// at a time within each section.
pub(crate) fn sectioned_import_runs(sections: &Sections, body: &[Stmt]) -> Vec<Range<usize>> {
    sections
        .ranges()
        .iter()
        .flat_map(|section| {
            runs_where(&body[section.clone()], is_import)
                .into_iter()
                .map(move |run| section.start + run.start..section.start + run.end)
        })
        .collect()
}

/// True when an own-line comment sits on the line directly above
/// `stmt`, describing the statement a whole-line deletion removes.
fn comment_leads(source: &Source, stmt: TextRange) -> bool {
    let text = source.text();
    let line_start = text.line_start(stmt.start());
    line_start > TextSize::default()
        && leading_comment_block(
            source,
            text.line_start(line_start - TextSize::from(1)),
            line_start,
        )
        .is_some()
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

/// True when `stmt` holds its lines alone, carrying only whitespace
/// ahead of it and only whitespace or a trailing comment behind it.
fn stands_alone(source: &Source, stmt: TextRange) -> bool {
    let lines = source.text().full_lines_range(stmt);
    let before = source.slice(TextRange::new(lines.start(), stmt.start()));
    let after = source
        .slice(TextRange::new(stmt.end(), lines.end()))
        .trim_start();
    before.trim().is_empty() && (after.is_empty() || after.starts_with('#'))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::primitives::edit::apply_edits;
    use crate::primitives::orderer::member_blocks;
    use crate::testing::parse;

    #[rstest]
    #[case("from __future__ import annotations\n", true)]
    #[case("from __future__ import annotations, division\n", true)]
    #[case("from __future__ import division\n", false)]
    #[case("from other import annotations\n", false)]
    #[case("import __future__\n", false)]
    #[case("x = 1\n", false)]
    fn defers_annotations_detects_the_future_import(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        assert_eq!(defers_annotations(&source.ast().body), expected);
    }

    #[rstest]
    #[case("import os\nimport sys\n", true, Some(0))]
    #[case("import os\nfrom collections import deque\n", true, Some(1))]
    #[case("import os\nfrom collections import deque\n", false, Some(0))]
    #[case("import os\nimport sys\n", false, Some(0))]
    #[case("from __future__ import annotations\nimport os\n", true, Some(1))]
    #[case("from __future__ import annotations\nimport os\n", false, Some(0))]
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

    #[rstest]
    #[case::sole_alias("from typing import Optional\n", &[0], "")]
    #[case::sole_alias_with_trailing_comment("from typing import Optional  # noqa\n", &[0], "")]
    #[case::every_alias("from typing import Optional, cast\n", &[0, 1], "")]
    #[case::leading("from typing import Optional, cast\n", &[0], "from typing import cast\n")]
    #[case::trailing("from typing import cast, Optional\n", &[1], "from typing import cast\n")]
    #[case::interior("from typing import a, b, c\n", &[1], "from typing import a, c\n")]
    #[case::leading_run("from typing import a, b, c\n", &[0, 1], "from typing import c\n")]
    #[case::trailing_run("from typing import a, b, c\n", &[1, 2], "from typing import a\n")]
    #[case::scattered("from typing import a, b, c\n", &[0, 2], "from typing import b\n")]
    #[case::nothing_dropped("from typing import a, b\n", &[], "from typing import a, b\n")]
    #[case::bare_import("import typing, os\n", &[0], "import os\n")]
    #[case::leads_its_line("import typing; x = 1\n", &[0], "import typing; x = 1\n")]
    #[case::follows_other_code("x = 1; import typing\n", &[0], "x = 1; import typing\n")]
    #[case::parenthesized(
        "from typing import (\n    a,\n    b,\n)\n",
        &[0],
        "from typing import (\n    b,\n)\n"
    )]
    #[case::interior_comment(
        "from typing import (\n    a,\n    # keep b around\n    b,\n)\n",
        &[0],
        "from typing import (\n    a,\n    # keep b around\n    b,\n)\n"
    )]
    #[case::leading_comment_holds_the_statement(
        "# loaded for the side effect\nimport typing\n",
        &[0],
        "# loaded for the side effect\nimport typing\n"
    )]
    #[case::leading_comment_leaves_a_partial_prune_alone(
        "# the typing pair\nfrom typing import a, b\n",
        &[0],
        "# the typing pair\nfrom typing import b\n"
    )]
    fn prune_import_aliases_drops_each_run_with_the_separator_binding_it(
        #[case] src: &str,
        #[case] dropped: &[usize],
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let (stmt, names) = source
            .ast()
            .body
            .iter()
            .find_map(|stmt| match stmt {
                Stmt::Import(node) => Some((stmt, &node.names)),
                Stmt::ImportFrom(node) => Some((stmt, &node.names)),
                _ => None,
            })
            .expect("the source carries an import");
        let edits = prune_import_aliases(&source, stmt.range(), names, |i| !dropped.contains(&i));
        let pruned = apply_edits(source.text(), edits).expect("the deletions do not overlap");
        assert_eq!(pruned, expected);
    }

    #[test]
    fn sectioned_import_runs_offsets_each_section_run_past_the_divider() {
        let source = parse("import os\nimport sys\n# --- Typing ---\nimport abc\nimport io\n");
        let body = &source.ast().body;
        let blocks = member_blocks(&source, body, source.module_range());
        let sections = Sections::of(&source, &blocks);
        assert_eq!(sectioned_import_runs(&sections, body), vec![0..2, 2..4]);
    }
}

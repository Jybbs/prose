//! Classifies import statements into the canonical group order
//! `__future__` → bare → external `from` → local-package, finds the runs
//! of adjacent imports the ordering rules act on, builds the composite
//! sort key ordering a run within and across those groups, counts the
//! canonical blank lines dividing two imports, and shapes the deletions
//! that drop the aliases a rule has left unread. First-party detection
//! reads the package-name list from `[tool.prose.imports]`.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, HashSet},
    ops::Range,
};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{Alias, Stmt, StmtImportFrom};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{
        blanks::whitespace_start_before,
        comments::comment_leads,
        edit::{apply_inline_edits, whole_line_deletion},
        range::dropped_member_spans,
        sections::Sections,
        slots::runs_where,
    },
    source::Source,
};

const FUTURE_ANNOTATIONS: &str = "annotations";
const FUTURE_MODULE: &str = "__future__";

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

/// One import statement and the alias positions a rule drops from it.
pub(crate) struct Dropping<'a> {
    pub(crate) dropped: Vec<usize>,
    pub(crate) names: &'a [Alias],
    pub(crate) range: TextRange,
    pub(crate) slot: usize,
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
/// `space-statements` share. `Some(1)` divides distinct groups while
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

/// The body slot whose import the drop of a comment-led `slot` lands
/// on, so the comment heads that import once the later rules have laid
/// the block out. Within the run of `runs` holding `slot`, a sibling
/// `survives` the drops, opens its own line with no comment leading it,
/// and is the next one sharing the module `merges` folds the statement
/// into, or the next one at all when `slot` heads its band as written
/// and `sorted_heads` seats another member first, since
/// `band-constants` then reseats the comment over the sorted head as
/// the band's heading. `sorted_heads` pairs with `runs` and is empty
/// when no band sorts them. `None` leaves the drop held under its
/// comment.
pub(crate) fn fold_landing(
    source: &Source,
    body: &[Stmt],
    runs: &[Vec<usize>],
    sorted_heads: &[usize],
    merges: bool,
    slot: usize,
    survives: impl Fn(usize) -> bool,
) -> Option<usize> {
    let (index, run) = runs.iter().find_position(|run| run.contains(&slot))?;
    let landable = |other: usize| {
        other > slot
            && survives(other)
            && stands_alone(source, body[other].range())
            && !comment_leads(source, body[other].start())
    };
    let module = body[slot].as_import_from_stmt().map(module_key);
    let same_module = |other: usize| {
        module.is_some() && body[other].as_import_from_stmt().map(module_key) == module
    };
    let mut after = run.iter().copied().filter(|&other| landable(other));
    if merges && let Some(sibling) = after.clone().find(|&other| same_module(other)) {
        return Some(sibling);
    }
    let reseated = sorted_heads
        .get(index)
        .is_some_and(|&head| run[0] == slot && head != slot);
    reseated.then(|| after.next()).flatten()
}

/// True for an absolute `from __future__ import …` statement.
/// Display width of the `import ` keyword and its trailing space, the
/// distance from an aligned `import` column to the first member.
pub(crate) const IMPORT_KEYWORD_WIDTH: usize = "import ".len();

/// The widest member a `from`-import carries, `None` for a statement
/// holding one member alone, which `reflow-imports` never splits. A
/// split roster packs one member per row at the narrowest, so this is
/// the widest row the statement can reach past its prefix.
pub(crate) fn widest_member_width(source: &Source, stmt: &Stmt) -> Option<usize> {
    let node = stmt.as_import_from_stmt()?;
    let [_, _, ..] = node.names.as_slice() else {
        return None;
    };
    node.names
        .iter()
        .map(|alias| source.slice(alias.range()).width())
        .max()
}

pub(crate) fn is_future(node: &StmtImportFrom) -> bool {
    node.level == 0 && node.module.as_deref() == Some(FUTURE_MODULE)
}

/// True for an `import` or `from`-import statement.
pub(crate) fn is_import(stmt: &Stmt) -> bool {
    stmt.is_import_stmt() || stmt.is_import_from_stmt()
}

/// The module a `from`-import reads, its leading-dot count beside the
/// module name, what tells one such import's module from another's.
pub(crate) fn module_key(node: &StmtImportFrom) -> ModuleKey<'_> {
    (node.level, node.module.as_deref())
}

/// The deletions dropping every alias of an import statement that
/// `keep` rejects, empty when every alias survives, when the statement
/// shares its lines with other code, or when a comment sits inside it.
///
/// A statement losing all of its aliases goes whole, taking its full
/// lines, unless an own-line comment block leads it, since the deletion
/// would strand that block, where `folded` marks a statement whose
/// drop lands on an import the block heads once the later rules have
/// laid the block out. One losing a subset keeps the survivors
/// byte-for-byte, each deletion covering one run of dropped aliases
/// together with the separator binding it to the survivor beside it.
pub(crate) fn prune_import_aliases(
    source: &Source,
    stmt: TextRange,
    names: &[Alias],
    folded: bool,
    keep: impl Fn(usize) -> bool,
) -> Vec<Edit> {
    let kept = (0..names.len()).filter(|&index| keep(index)).count();
    if kept == names.len() || !stands_alone(source, stmt) || source.intersects_comment(stmt) {
        return Vec::new();
    }
    if kept == 0 {
        return if comment_leads(source, stmt.start()) && !folded {
            Vec::new()
        } else {
            vec![whole_line_deletion(source, stmt)]
        };
    }
    let members: Vec<TextRange> = names.iter().map(|alias| alias.range).collect();
    dropped_member_spans(&members, |index| !keep(index))
        .into_iter()
        .map(Edit::range_deletion)
        .collect()
}

/// One fix group per statement of `drops` losing an alias, the drops
/// of `body`'s module-scope imports. A statement losing every alias
/// under a leading comment gives its line to the import `landing`
/// names for it, that import's own drops applied to the text it moves
/// and its former lines cleared along with the blank run above them,
/// so the comment heads the import its block reads over. Of two such
/// statements landing on one import, the later takes it and the
/// earlier drops whole beneath its own comment, which then heads the
/// moved line.
pub(crate) fn prune_import_statements(
    source: &Source,
    body: &[Stmt],
    drops: &[Dropping],
    landing: impl Fn(usize, &dyn Fn(usize) -> bool) -> Option<usize>,
) -> Vec<Vec<Edit>> {
    let whole: HashSet<usize> = drops
        .iter()
        .filter(|drop| drop.dropped.len() == drop.names.len())
        .map(|drop| drop.slot)
        .collect();
    let survives = |slot: usize| !whole.contains(&slot);
    let landings: BTreeMap<usize, usize> = drops
        .iter()
        .filter(|drop| whole.contains(&drop.slot) && comment_leads(source, drop.range.start()))
        .filter_map(|drop| landing(drop.slot, &survives).map(|onto| (drop.slot, onto)))
        .collect();
    let claims: HashMap<usize, usize> =
        landings.iter().map(|(&lead, &onto)| (onto, lead)).collect();
    let edits_of = |drop: &Dropping| {
        prune_import_aliases(
            source,
            drop.range,
            drop.names,
            landings.contains_key(&drop.slot),
            |index| !drop.dropped.contains(&index),
        )
    };
    let line_span =
        |range: TextRange| TextRange::new(range.start(), source.text().line_end(range.end()));
    let mut consumed = HashSet::new();
    let groups: Vec<(usize, Vec<Edit>)> = drops
        .iter()
        .map(|drop| {
            let edits = edits_of(drop);
            let Some(&onto) = landings
                .get(&drop.slot)
                .filter(|&&onto| claims[&onto] == drop.slot && !edits.is_empty())
            else {
                return (drop.slot, edits);
            };
            let span = line_span(body[onto].range());
            let text = drops.iter().find(|other| other.slot == onto).map_or_else(
                || source.slice(span).to_owned(),
                |sibling| apply_inline_edits(source, span, &edits_of(sibling)).into_owned(),
            );
            consumed.insert(onto);
            (
                drop.slot,
                vec![
                    Edit::range_replacement(text, line_span(drop.range)),
                    Edit::range_deletion(lines_under_blank_run(source, body[onto].range())),
                ],
            )
        })
        .collect();
    groups
        .into_iter()
        .filter(|(slot, edits)| !consumed.contains(slot) && !edits.is_empty())
        .map(|(_, edits)| edits)
        .collect()
}

/// Slot ranges of every import run across a sectioned body, each run
/// offset to absolute slot indices so it never spans a section divider.
/// The unit `group-imports` partitions and `alphabetize-siblings`
/// sorts, one run at a time within each section.
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

/// True when the root package of `name` (the substring up to the
/// first `.`) appears in `first_party`.
fn is_first_party(name: &str, first_party: &[String]) -> bool {
    let root = name.split_once('.').map_or(name, |(root, _)| root);
    first_party.iter().any(|p| p == root)
}

/// The full lines `stmt` sits on together with the blank run directly
/// above them, held within the statement's notebook cell.
fn lines_under_blank_run(source: &Source, stmt: TextRange) -> TextRange {
    let lines = source.full_lines_within_cell(stmt);
    let above = whitespace_start_before(source, lines.start());
    TextRange::new(
        source.text().full_line_end(above).min(lines.start()),
        lines.end(),
    )
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
pub(crate) fn stands_alone(source: &Source, stmt: TextRange) -> bool {
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
    use ruff_text_size::Ranged;

    use super::*;
    use crate::{
        primitives::{edit::apply_edits, orderer::member_blocks, slots::slot_runs},
        testing::parse,
    };

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
    #[case::leading_comment_across_a_blank_holds_the_statement(
        "# loaded for the side effect\n\nimport typing\n",
        &[0],
        "# loaded for the side effect\n\nimport typing\n"
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
        let edits = prune_import_aliases(&source, stmt.range(), names, false, |i| {
            !dropped.contains(&i)
        });
        let pruned = apply_edits(source.text(), edits).expect("the deletions do not overlap");
        assert_eq!(pruned, expected);
    }

    /// The import runs of `source`'s module body as written.
    fn runs(source: &Source) -> Vec<Vec<usize>> {
        slot_runs(&source.ast().body, |a, b| is_import(a) && is_import(b))
            .filter(|run| is_import(&source.ast().body[run.start]))
            .map(Iterator::collect)
            .collect()
    }

    #[rstest]
    #[case::same_module_sibling("# c\nfrom p import a\nfrom q import x\nfrom p import b\n", &[], true, Some(2))]
    #[case::merges_off("# c\nfrom p import a\nfrom q import x\nfrom p import b\n", &[], false, None)]
    #[case::band_head_reseated("# c\nfrom .p import a\nfrom ..q import x\n", &[1], false, Some(1))]
    #[case::band_head_sorts_first("# c\nfrom ..p import a\nfrom .q import x\n", &[0], false, None)]
    #[case::not_the_head("from ..q import x\n# c\nfrom .p import a\nfrom .r import y\n", &[0], false, None)]
    #[case::sibling_led_by_a_comment("# c\nfrom p import a\n# d\nfrom p import b\n", &[], true, None)]
    #[case::sibling_sharing_its_line("# c\nfrom p import a\nfrom p import b; x = 1\n", &[], true, None)]
    #[case::lone_import("# c\nfrom p import a\nx = 1\n", &[], true, None)]
    fn fold_landing_names_the_import_the_comment_heads_next(
        #[case] src: &str,
        #[case] sorted_heads: &[usize],
        #[case] merges: bool,
        #[case] expected: Option<usize>,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        let slot = body
            .iter()
            .position(|stmt| {
                stmt.as_import_from_stmt()
                    .is_some_and(|node| node.names[0].name.as_str() == "a")
            })
            .expect("the `a` import");
        let landing = fold_landing(
            &source,
            body,
            &runs(&source),
            sorted_heads,
            merges,
            slot,
            |_| true,
        );
        assert_eq!(landing, expected);
    }

    #[test]
    fn fold_landing_skips_a_sibling_the_drops_take() {
        let source = parse("# c\nfrom p import a\nfrom p import b\nfrom p import d\n");
        let body = &source.ast().body;
        let landing = fold_landing(&source, body, &runs(&source), &[], true, 0, |slot| {
            slot != 1
        });
        assert_eq!(landing, Some(2));
    }

    #[rstest]
    #[case::moves_the_sibling_up(
        "# c\nfrom p import a\n\nfrom p import b\n",
        &[(0, &[0][..]), (1, &[][..])],
        "# c\nfrom p import b\n"
    )]
    #[case::applies_the_siblings_own_drops(
        "# c\nfrom p import a\nfrom p import b, d  # t\n",
        &[(0, &[0][..]), (1, &[0][..])],
        "# c\nfrom p import d  # t\n"
    )]
    #[case::later_lead_takes_the_landing(
        "# c\nfrom p import a\n# d\nfrom p import b\nfrom p import d\n",
        &[(0, &[0][..]), (1, &[0][..])],
        "# c\n# d\nfrom p import d\n"
    )]
    #[case::no_landing_holds_the_lead(
        "# c\nfrom p import a\nfrom p import b\n",
        &[(0, &[0][..]), (1, &[0][..])],
        "# c\nfrom p import a\n"
    )]
    #[case::uncommented_lead_drops_whole(
        "from p import a\nfrom p import b\n",
        &[(0, &[0][..])],
        "from p import b\n"
    )]
    fn prune_import_statements_lands_a_commented_drop_on_the_next_import(
        #[case] src: &str,
        #[case] drops: &[(usize, &[usize])],
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        let drops: Vec<Dropping> = drops
            .iter()
            .map(|&(slot, dropped)| Dropping {
                dropped: dropped.to_vec(),
                names: &body[slot]
                    .as_import_from_stmt()
                    .expect("a from-import")
                    .names,
                range: body[slot].range(),
                slot,
            })
            .collect();
        let runs = runs(&source);
        let groups = prune_import_statements(&source, body, &drops, |slot, survives| {
            fold_landing(&source, body, &runs, &[], true, slot, survives)
        });
        let pruned = apply_edits(source.text(), groups.concat()).expect("the edits weave");
        assert_eq!(pruned, expected);
    }

    #[test]
    fn prune_import_aliases_drops_a_commented_statement_a_merge_folds() {
        let source = parse("# local imports\nfrom pkg import a\nfrom pkg import b\n");
        let stmt = &source.ast().body[0];
        let names = &stmt.as_import_from_stmt().expect("a from-import").names;
        let edits = prune_import_aliases(&source, stmt.range(), names, true, |_| false);
        let pruned = apply_edits(source.text(), edits).expect("the deletion stands alone");
        assert_eq!(pruned, "# local imports\nfrom pkg import b\n");
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

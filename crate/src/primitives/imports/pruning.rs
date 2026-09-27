//! Drops the imports and aliases a module never reads, and finds the
//! import a dropped statement's comment lands on.

use std::collections::BTreeMap;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{Alias, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use super::*;
use crate::{
    primitives::{
        comments::comment_leads,
        edit::{apply_inline_edits, whole_line_deletions},
        range::dropped_member_spans,
    },
    rules::RuleId,
    source::Source,
};

/// One import statement and the alias positions a rule drops from it.
pub(crate) struct Dropping<'a> {
    pub(crate) dropped: Vec<usize>,
    pub(crate) names: &'a [Alias],
    pub(crate) range: TextRange,
    pub(crate) slot: usize,
}

impl Dropping<'_> {
    /// True when every alias of the statement drops.
    fn drops_every_alias(&self) -> bool {
        self.dropped.len() == self.names.len()
    }

    /// Returns the pruning that drops the aliases at `dropped`, deleting
    /// nothing when none drops, when the statement shares its lines with
    /// other code, or when a comment sits inside it. A statement losing
    /// every alias drops whole unless a leading comment block holds it and
    /// `folded` is false, and one losing a subset drops each run of dropped
    /// aliases with the separator binding it.
    fn pruning(&self, source: &Source, folded: bool) -> Pruning {
        if self.dropped.is_empty()
            || !stands_alone(source, self.range)
            || !source
                .comment_ranges()
                .comments_in_range(self.range)
                .is_empty()
        {
            return Pruning::Members(Vec::new());
        }
        if self.drops_every_alias() {
            return if comment_leads(source, self.range.start()) && !folded {
                Pruning::Members(Vec::new())
            } else {
                Pruning::Whole
            };
        }
        let members: Vec<TextRange> = self.names.iter().map(|alias| alias.range).collect();
        Pruning::Members(
            dropped_member_spans(&members, |index| self.dropped.contains(&index))
                .into_iter()
                .map(Edit::range_deletion)
                .collect(),
        )
    }
}

/// What pruning makes of one import statement.
enum Pruning {
    /// The deletions dropping a subset of the aliases, empty where the
    /// statement stays as written.
    Members(Vec<Edit>),
    /// The statement's lines clear whole.
    Whole,
}

/// The body slot whose import a comment-led `slot`'s drop lands on.
/// Within the run of `runs` holding `slot`, the landing sibling comes
/// later, `survives` the drops, holds its line alone with no comment
/// leading it, and shares `slot`'s module where `merges` is set. Where
/// `heads`, each band's head beside the member its sort seats first,
/// names `slot` as the head of a band whose sort seats another member
/// first, the landing is that member, or the next such sibling where it
/// cannot take the line. `None` leaves the drop held under its comment.
pub(crate) fn fold_landing(
    source: &Source,
    body: &[Stmt],
    runs: &[Vec<usize>],
    heads: &[(usize, usize)],
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
    let (head, sorted) = *heads.get(index)?;
    (head == slot && sorted != slot)
        .then(|| {
            Some(sorted)
                .filter(|&sorted| landable(sorted))
                .or_else(|| after.next())
        })
        .flatten()
}

/// One fix group per statement of `drops` losing an alias, the drops
/// of `body`'s module-scope imports. A statement losing every alias
/// under a leading comment gives its line to the import `landing`
/// names, and of two statements landing on one import the later takes
/// it. The lines of every statement dropping whole and of every import
/// a drop lands on clear together per [`whole_line_deletions`], leaving
/// out a statement a suppression of `rule` pins, which neither lands nor
/// clears.
pub(crate) fn prune_import_statements(
    source: &Source,
    body: &[Stmt],
    drops: &[Dropping],
    rule: RuleId,
    landing: impl Fn(usize, &dyn Fn(usize) -> bool) -> Option<usize>,
) -> Vec<Vec<Edit>> {
    let whole: FxHashSet<usize> = drops
        .iter()
        .filter(|drop| drop.drops_every_alias())
        .map(|drop| drop.slot)
        .collect();
    let survives = |slot: usize| !whole.contains(&slot);
    let pinned = |slot: usize| source.suppression_map().pins(body[slot].range(), rule);
    let landings: BTreeMap<usize, usize> = drops
        .iter()
        .filter(|drop| drop.drops_every_alias() && comment_leads(source, drop.range.start()))
        .filter_map(|drop| {
            landing(drop.slot, &survives)
                .filter(|&onto| !pinned(drop.slot) && !pinned(onto))
                .map(|onto| (drop.slot, onto))
        })
        .collect();
    let claims: FxHashMap<usize, usize> =
        landings.iter().map(|(&lead, &onto)| (onto, lead)).collect();
    let pruning_of = |drop: &Dropping| drop.pruning(source, landings.contains_key(&drop.slot));
    let line_span =
        |range: TextRange| TextRange::new(range.start(), source.text().line_end(range.end()));
    let mut consumed = FxHashSet::default();
    let mut cleared = BTreeMap::new();
    let mut groups: Vec<(usize, Vec<Edit>)> = drops
        .iter()
        .enumerate()
        .map(|(index, drop)| {
            if let Pruning::Members(edits) = pruning_of(drop) {
                return (drop.slot, edits);
            }
            let Some(&onto) = landings
                .get(&drop.slot)
                .filter(|&&onto| claims[&onto] == drop.slot)
            else {
                if !pinned(drop.slot) {
                    cleared.insert(drop.slot, index);
                }
                return (drop.slot, Vec::new());
            };
            let span = line_span(body[onto].range());
            let text = drops.iter().find(|other| other.slot == onto).map_or_else(
                || source.slice(span).to_owned(),
                |sibling| {
                    let Pruning::Members(edits) = pruning_of(sibling) else {
                        unreachable!("invariant: the import a drop lands on survives the drops");
                    };
                    apply_inline_edits(source, span, &edits).into_owned()
                },
            );
            consumed.insert(onto);
            cleared.insert(onto, index);
            (
                drop.slot,
                vec![Edit::range_replacement(text, line_span(drop.range))],
            )
        })
        .collect();
    let deletions = whole_line_deletions(source, cleared.keys().map(|&slot| body[slot].range()));
    for (&index, deletion) in cleared.values().zip(deletions) {
        groups[index].1.push(deletion);
    }
    groups
        .into_iter()
        .filter(|(slot, edits)| !consumed.contains(slot) && !edits.is_empty())
        .map(|(_, edits)| edits)
        .collect()
}

/// True when `stmt` holds its lines alone, carrying only whitespace
/// ahead of it and only whitespace or a trailing comment behind it. A
/// row a `\` join continues belongs to the row above and never stands
/// alone.
pub(crate) fn stands_alone(source: &Source, stmt: TextRange) -> bool {
    if source.continues_a_logical_line(stmt.start()) {
        return false;
    }
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
    use crate::{
        rules::prune_inert_imports::PruneInertImports,
        testing::{applied_text, parse},
    };

    /// The drop of the aliases at `dropped` from the first import of
    /// `source`'s module body.
    fn first_import_dropping<'a>(source: &'a Source, dropped: &[usize]) -> Dropping<'a> {
        let (slot, names) = source
            .ast()
            .body
            .iter()
            .enumerate()
            .find_map(|(slot, stmt)| match stmt {
                Stmt::Import(node) => Some((slot, &node.names)),
                Stmt::ImportFrom(node) => Some((slot, &node.names)),
                _ => None,
            })
            .expect("the source carries an import");
        Dropping {
            dropped: dropped.to_vec(),
            names,
            range: source.ast().body[slot].range(),
            slot,
        }
    }

    /// The import runs of `source`'s module body as written.
    fn merge_runs(source: &Source) -> Vec<Vec<usize>> {
        import_runs(&source.ast().body)
    }

    /// The text `source` reads once the pruning of `drop` applies.
    fn pruned_text(source: &Source, drop: &Dropping, folded: bool) -> String {
        let edits = match drop.pruning(source, folded) {
            Pruning::Members(edits) => edits,
            Pruning::Whole => whole_line_deletions(source, [drop.range]),
        };
        applied_text(source, edits)
    }

    #[rstest]
    #[case::same_module_sibling("# c\nfrom p import a\nfrom q import x\nfrom p import b\n", &[], true, Some(2))]
    #[case::merges_off("# c\nfrom p import a\nfrom q import x\nfrom p import b\n", &[], false, None)]
    #[case::band_head_reseated("# c\nfrom .p import a\nfrom ..q import x\n", &[(0, 1)], false, Some(1))]
    #[case::band_head_sorts_first("# c\nfrom ..p import a\nfrom .q import x\n", &[(0, 0)], false, None)]
    #[case::not_the_head("from ..q import x\n# c\nfrom .p import a\nfrom .r import y\n", &[(0, 0)], false, None)]
    #[case::grouped_head_lands_on_the_sorted_head(
        "from ..q import x\n# c\nfrom .p import a\nfrom .r import y\nfrom .s import z\n",
        &[(1, 3)],
        false,
        Some(3),
    )]
    #[case::sorted_head_led_by_a_comment(
        "# c\nfrom .p import a\nfrom .q import x\n# d\nfrom .r import y\n",
        &[(0, 2)],
        false,
        Some(1),
    )]
    #[case::sibling_led_by_a_comment("# c\nfrom p import a\n# d\nfrom p import b\n", &[], true, None)]
    #[case::sibling_sharing_its_line("# c\nfrom p import a\nfrom p import b; x = 1\n", &[], true, None)]
    #[case::lone_import("# c\nfrom p import a\nx = 1\n", &[], true, None)]
    fn fold_landing_names_the_import_the_comment_heads_next(
        #[case] src: &str,
        #[case] heads: &[(usize, usize)],
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
            &merge_runs(&source),
            heads,
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
        let landing = fold_landing(&source, body, &merge_runs(&source), &[], true, 0, |slot| {
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
    #[case::moved_import_keeps_the_wider_run(
        "# c\nfrom p import a\n\nfrom p import b\nfrom p import d\n",
        &[(0, &[0][..])],
        "# c\nfrom p import b\n\nfrom p import d\n"
    )]
    #[case::skipped_drop_stays_out_of_the_block(
        "x = 1\n\n\nfrom p import a\n\nfrom q import b  # prose: skip\n\ny = 2\n",
        &[(1, &[0][..]), (2, &[0][..])],
        "x = 1\n\n\nfrom q import b  # prose: skip\n\ny = 2\n"
    )]
    #[case::moved_import_clears_one_block_with_a_whole_drop(
        "# c\nfrom p import a\n\n\nfrom q import u\n\nfrom p import b\n\n\nx = 1\n",
        &[(0, &[0][..]), (1, &[0][..])],
        "# c\nfrom p import b\n\n\nx = 1\n"
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
        let runs = merge_runs(&source);
        let groups = prune_import_statements(
            &source,
            body,
            &drops,
            PruneInertImports::SLUG,
            |slot, survives| fold_landing(&source, body, &runs, &[], true, slot, survives),
        );
        let pruned = applied_text(&source, groups.concat());
        assert_eq!(pruned, expected);
    }

    #[test]
    fn pruning_drops_a_commented_statement_a_merge_folds() {
        let source = parse("# local imports\nfrom pkg import a\nfrom pkg import b\n");
        let pruned = pruned_text(&source, &first_import_dropping(&source, &[0]), true);
        assert_eq!(pruned, "# local imports\nfrom pkg import b\n");
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
    fn pruning_drops_each_run_with_the_separator_binding_it(
        #[case] src: &str,
        #[case] dropped: &[usize],
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let pruned = pruned_text(&source, &first_import_dropping(&source, dropped), false);
        assert_eq!(pruned, expected);
    }
}

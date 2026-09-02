//! The runs a merge gathers within and the fold forecasts other rules
//! read: the import runs as written or as the bands `band-constants`
//! sorts, the merge groups they hold, and the landing a comment-led
//! drop takes once a fold clears its statement.

use std::cell::OnceCell;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, StmtImportFrom};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use super::own_line_indent;
use crate::{
    config::Config,
    primitives::{
        comments::comments_held_by,
        imports::{
            Dropping, ModuleKey, defers_annotations, fold_landing, import_runs, is_star,
            module_key, prune_import_statements, stands_alone,
        },
        orderer::member_blocks,
    },
    rules::band_constants::{BandConstants, Bands, Carry},
    source::Source,
};

/// The same-module merges `reflow-imports` makes and the import bands
/// `band-constants` heads, forecast by a rule seated ahead of both
/// whose drop of a comment-led statement then lands on the import the
/// comment reads over.
#[derive(Debug)]
pub(crate) struct Folds {
    bands: Option<BandConstants>,
    merges: bool,
}

impl Folds {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.reflow_imports;
        Self {
            bands: band_forecast(config),
            merges: rules.enabled && rules.merge_members,
        }
    }

    /// `band-constants` as this pipeline runs it, `None` when the rule
    /// is off.
    pub(crate) fn bands(&self) -> Option<&BandConstants> {
        self.bands.as_ref()
    }

    /// The module-body slot whose import the drop of `slot` lands on,
    /// per [`fold_landing`], `None` when no later rule carries the
    /// comment onto a sibling that `survives` the drops.
    pub(crate) fn landing(
        &self,
        source: &Source,
        slot: usize,
        survives: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let body = &source.ast().body;
        let runs = MergeRuns::of(
            self.bands.as_ref(),
            source,
            body,
            source.module_range(),
            |_| true,
        );
        fold_landing(
            source,
            body,
            &runs.runs,
            &runs.sorted_heads,
            self.merges,
            slot,
            survives,
        )
    }

    /// One fix group per statement of `drops`, a comment-led statement
    /// losing every alias landing on the import this forecast names.
    pub(crate) fn prune(&self, source: &Source, drops: &[Dropping]) -> Vec<Vec<Edit>> {
        prune_import_statements(source, &source.ast().body, drops, |slot, survives| {
            self.landing(source, slot, survives)
        })
    }
}

/// The runs a body's merges gather within, beside the member blocks a
/// comment between two members is read against, built on first use
/// through [`Self::blocks`]. `banded` marks runs that are the bands
/// `band-constants` sorts, `carries` then holding every comment the
/// banding moves between members.
pub(super) struct MergeRuns {
    pub(super) banded: bool,
    blocks: OnceCell<Vec<TextRange>>,
    pub(super) carries: Vec<Carry>,
    pub(super) runs: Vec<Vec<usize>>,
    /// The slot each band's sort seats first, in step with `runs` and
    /// empty where the runs are the imports as written.
    pub(super) sorted_heads: Vec<usize>,
}

impl MergeRuns {
    /// The import runs of `body` as written, or the bands `bands`
    /// forecasts once it hoists the constants between two runs and
    /// sorts each, sought where a module repeats across the runs as
    /// written or where `seek` reads them as needed over the runs as
    /// written.
    pub(super) fn of(
        bands: Option<&BandConstants>,
        source: &Source,
        body: &[Stmt],
        outer: TextRange,
        seek: impl FnOnce(&[Vec<usize>]) -> bool,
    ) -> Self {
        let runs = import_runs(body);
        let joined = bands
            .filter(|_| seek(&runs) || repeats_across(source, body, &runs))
            .and_then(|rule| {
                rule.forecast(source, body, outer, defers_annotations(&source.ast().body))
            });
        match joined {
            Some(Bands {
                blocks,
                carries,
                imports,
                ..
            }) => {
                let (runs, sorted_heads) = imports
                    .into_iter()
                    .map(|band| (band.slots, band.sorted_head))
                    .unzip();
                Self {
                    banded: true,
                    blocks: OnceCell::from(blocks),
                    carries,
                    runs,
                    sorted_heads,
                }
            }
            None => Self {
                banded: false,
                blocks: OnceCell::new(),
                carries: Vec::new(),
                runs,
                sorted_heads: Vec::new(),
            },
        }
    }

    /// The member blocks of `body` through the statement after its
    /// last import, every slot the runs and the gaps between them
    /// read, built on first use.
    pub(super) fn blocks(&self, source: &Source, body: &[Stmt], outer: TextRange) -> &[TextRange] {
        self.blocks.get_or_init(|| {
            let reach = self
                .runs
                .iter()
                .flatten()
                .max()
                .map_or(0, |&last| (last + 2).min(body.len()));
            member_blocks(source, &body[..reach], outer)
        })
    }
}

/// `band-constants` as configured, `None` when the rule is off.
pub(super) fn band_forecast(config: &Config) -> Option<BandConstants> {
    config
        .rules
        .band_constants
        .enabled
        .then(|| BandConstants::from_config(config))
}

/// True when a comment sits within or beside one of `runs`, from the
/// end of the statement before the run to the start of the one after
/// it, the reach a banding carries a comment across.
pub(super) fn comments_beside(
    source: &Source,
    body: &[Stmt],
    outer: TextRange,
    runs: &[Vec<usize>],
) -> bool {
    runs.iter().any(|run| {
        let (first, last) = (run[0], run[run.len() - 1]);
        let lower = first
            .checked_sub(1)
            .map_or(outer.start(), |prev| body[prev].end());
        let upper = body.get(last + 1).map_or(outer.end(), Ranged::start);
        source.intersects_comment(TextRange::new(lower, upper))
    })
}

/// Slot groups of two or more mergeable `from`-imports sharing one
/// module within one of `runs`, each group spanning one notebook cell
/// and gathering cleanly per [`gathers_cleanly`].
pub(super) fn module_groups(
    source: &Source,
    body: &[Stmt],
    outer: TextRange,
    runs: &MergeRuns,
) -> Vec<Vec<usize>> {
    let mut groups = Vec::new();
    for run in &runs.runs {
        let mut by_module: Vec<(ModuleKey, Vec<usize>)> = Vec::new();
        for &slot in run {
            let Some(node) = body[slot]
                .as_import_from_stmt()
                .filter(|n| mergeable(source, n))
            else {
                continue;
            };
            let key = module_key(node);
            match by_module.iter_mut().find(|(seen, _)| *seen == key) {
                Some((_, slots)) => slots.push(slot),
                None => by_module.push((key, vec![slot])),
            }
        }
        groups.extend(
            by_module
                .into_iter()
                .map(|(_, slots)| slots)
                .filter(|slots| gathers_cleanly(source, body, outer, slots, runs)),
        );
    }
    groups
}

/// True when the members at `slots` fold into one statement without
/// disturbing their surroundings, being two or more statements of one
/// notebook cell whose span from first to last carries no comment
/// outside the block of a statement between them, which stays in place
/// while the gather clears the member lines.
fn gathers_cleanly(
    source: &Source,
    body: &[Stmt],
    outer: TextRange,
    slots: &[usize],
    runs: &MergeRuns,
) -> bool {
    let [first, .., last] = slots else {
        return false;
    };
    let reach = TextRange::new(body[*first].start(), body[*last].end());
    if !source.same_cell(reach.start(), reach.end()) {
        return false;
    }
    let span = source.full_lines_within_cell(reach);
    let comments = source.comment_ranges().comments_in_range(span);
    if comments.is_empty() {
        return true;
    }
    let blocks = runs.blocks(source, body, outer);
    comments_held_by(
        source,
        span,
        blocks,
        (*first + 1..*last).filter(|slot| !slots.contains(slot)),
    )
}

/// True when `node` can join a merged roster, being a single-line
/// `from`-import holding its line alone, so the fold clears no code
/// sharing it, and binding no star member, since `*` admits no sibling
/// on its statement.
fn mergeable(source: &Source, node: &StmtImportFrom) -> bool {
    own_line_indent(source, node).is_some()
        && stands_alone(source, node.range())
        && !node.names.iter().any(is_star)
}

/// True when a mergeable `from`-import's module recurs in a second run
/// of `runs`, the one shape a hoist between the runs would gather.
fn repeats_across(source: &Source, body: &[Stmt], runs: &[Vec<usize>]) -> bool {
    let mut seen: FxHashMap<ModuleKey, usize> = FxHashMap::default();
    runs.iter().enumerate().any(|(index, run)| {
        run.iter()
            .filter_map(|&slot| body[slot].as_import_from_stmt())
            .filter(|node| mergeable(source, node))
            .any(|node| *seen.entry(module_key(node)).or_insert(index) != index)
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{notebook, parse};

    #[rstest]
    #[case("import a\nimport b\nx = 1\n", false)]
    #[case("# heads the run\nimport a\nimport b\nx = 1\n", true)]
    #[case("import a\n# between\nimport b\n", true)]
    #[case("x = 1  # trailing\nimport a\n", true)]
    fn comments_beside_reads_the_reach_a_carry_crosses(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let body = &source.ast().body;
        let runs = import_runs(body);

        assert_eq!(
            comments_beside(&source, body, source.module_range(), &runs),
            expected,
        );
    }

    #[test]
    fn module_groups_decline_a_gather_across_a_cell_wall() {
        let source = notebook(&["from pkg import a\n", "from pkg import b\n"]);
        let body = &source.ast().body;
        let runs = MergeRuns::of(None, &source, body, source.module_range(), |_| true);

        assert!(module_groups(&source, body, source.module_range(), &runs).is_empty());
    }

    #[test]
    fn module_groups_gather_two_statements_of_one_module() {
        let source = parse("from pkg import a\nfrom pkg import b\n");
        let body = &source.ast().body;
        let runs = MergeRuns::of(None, &source, body, source.module_range(), |_| true);

        assert_eq!(
            module_groups(&source, body, source.module_range(), &runs),
            [vec![0, 1]],
        );
    }
}

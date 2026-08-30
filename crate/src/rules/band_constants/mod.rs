//! Module-scope constant banding. Hoists single-name assignments into a
//! leading band below the imports and a trailing band beneath the
//! definitions, declining whenever the assembled order would seat an
//! eager reference ahead of its definition. [`bander`] walks the module
//! body and each module-scope compound arm, applying the [`plan`]
//! analysis and emitting one fix group per banded body, or one per cell
//! over a notebook.

use ruff_diagnostics::Edit;
use ruff_python_ast::{PythonVersion, Stmt};
use ruff_text_size::TextRange;

use self::{analysis::module_band_plan, bander::band_module, plan::ImportBand};
use crate::{
    config::Config,
    primitives::{
        imports::defers_annotations,
        orderer::{any_sibling_shares_line, member_blocks},
        sections::Sections,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod analysis;
mod bander;
mod plan;

pub(crate) use self::plan::Carry;

pub(crate) struct BandConstants {
    code_width: usize,
    first_party: Vec<String>,
    group_imports: bool,
    group_subcategories: bool,
    max_tiers: Option<usize>,
    target_version: Option<PythonVersion>,
}

impl BandConstants {
    pub(crate) const MESSAGE: &'static str =
        "band module constants into leading and trailing bands";

    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.band_constants;
        Self {
            code_width: config.code_width(),
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            group_subcategories: rules.group_subcategories,
            max_tiers: rules.max_tiers.cap(),
            target_version: config.target_version,
        }
    }

    /// The import bands the rule sorts over `body`, forecast for a rule
    /// seated ahead of it, beside every comment the banding carries onto
    /// another member. A band holds the imports of one region once the
    /// hoist seats every constant between two of them below the run.
    /// `None` when a sibling shares a line or the plan declines the
    /// body.
    pub(crate) fn import_bands(
        &self,
        source: &Source,
        body: &[Stmt],
        outer: TextRange,
    ) -> Option<Bands> {
        if any_sibling_shares_line(source, body) {
            return None;
        }
        let blocks = member_blocks(source, body, outer);
        let sections = Sections::of(source, &blocks);
        let (imports, carries) = module_band_plan(
            source,
            body,
            &blocks,
            self,
            defers_annotations(&source.ast().body),
        )?
        .import_bands(body, &sections, self);
        Some(Bands {
            blocks,
            carries,
            imports,
        })
    }
}

#[cfg(test)]
impl Default for BandConstants {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

impl Rule for BandConstants {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        band_module(self, source)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The import bands forecast over one body beside the body's member
/// blocks and the comments the banding carries between members.
pub(crate) struct Bands {
    pub(crate) blocks: Vec<TextRange>,
    pub(crate) carries: Vec<Carry>,
    pub(crate) imports: Vec<ImportBand>,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::hoist_joins_the_runs("from p import a\nX = 1\nfrom q import b\n\nprint(a, b, X)\n", Some((vec![0, 2], 0)))]
    #[case::sort_reseats_the_head("from .p import a\nfrom ..q import b\n", Some((vec![0, 1], 1)))]
    #[case::pinned_anchor_splits_the_band("from p import a\nprint(a)\nfrom q import b\n", Some((vec![0], 0)))]
    #[case::shared_line_declines("from p import a; from q import b\n", None)]
    fn import_bands_reads_the_band_the_hoist_seats(
        #[case] src: &str,
        #[case] first: Option<(Vec<usize>, usize)>,
    ) {
        let source = parse(src);
        let rule = BandConstants::default();
        let bands = rule.import_bands(&source, &source.ast().body, source.module_range());
        assert_eq!(
            bands.and_then(|bands| {
                bands
                    .imports
                    .into_iter()
                    .next()
                    .map(|band| (band.slots, band.sorted_head))
            }),
            first,
        );
    }

    #[rstest]
    #[case::sort_reseats_the_heading("# heads the run\nfrom .p import a\nfrom ..q import b\n", Some((0, 1, false)))]
    #[case::run_binds_back_as_a_trailer("import os\n# documents os\n\nfrom x import y\n", Some((1, 0, true)))]
    #[case::run_binds_back_above_a_wide_line("import os\n# documents os at a length the line cannot take within the width\n\nfrom x import y\n", Some((1, 0, false)))]
    #[case::heading_stays_on_the_sorted_head(
        "# heads the run\nfrom ..q import b\nfrom .p import a\n",
        None
    )]
    fn import_bands_reads_the_comment_the_banding_carries(
        #[case] src: &str,
        #[case] first: Option<(usize, usize, bool)>,
    ) {
        let source = parse(src);
        let rule = BandConstants {
            code_width: 40,
            ..BandConstants::default()
        };
        let bands = rule.import_bands(&source, &source.ast().body, source.module_range());
        assert_eq!(
            bands.and_then(|bands| {
                bands
                    .carries
                    .first()
                    .map(|carry| (carry.absorbs, carry.carrier, carry.trails))
            }),
            first,
        );
    }
}

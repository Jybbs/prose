//! Module-scope constant banding. Hoists single-name assignments into a
//! leading band below the imports and a trailing band beneath the
//! definitions, declining whenever the assembled order would seat an
//! eager reference ahead of its definition. The rule walks the module
//! body and each module-scope compound arm, applying the [`plan`]
//! analysis and emitting one fix group per banded body, or one per cell
//! over a notebook.

use ruff_diagnostics::Edit;
use ruff_python_ast::{PythonVersion, Stmt};
use ruff_text_size::TextRange;

use self::{analysis::module_band_plan, bander::Bander, plan::ImportBand};
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

#[derive(Debug)]
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

    /// The order the rule seats `body` in, forecast for a rule seated
    /// ahead of it, beside the import bands it sorts and every comment
    /// the banding carries onto another member. A band holds the
    /// imports of one region once the hoist seats every constant
    /// between two of them below the run, and an annotation reads as
    /// deferred while `defer_annotations`. `None` when a sibling shares
    /// a line or the plan declines the body.
    pub(crate) fn forecast(
        &self,
        source: &Source,
        body: &[Stmt],
        outer: TextRange,
        defer_annotations: bool,
    ) -> Option<Bands> {
        if any_sibling_shares_line(source, body) {
            return None;
        }
        let blocks = member_blocks(source, body, outer);
        let sections = Sections::of(source, &blocks);
        let order: Vec<usize> = (0..body.len()).collect();
        Some(
            module_band_plan(source, body, &blocks, self, defer_annotations)?
                .forecast(body, blocks, &sections, self, &order),
        )
    }
}

impl Rule for BandConstants {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        Bander {
            defer_annotations: defers_annotations(body),
            rule: self,
            source,
        }
        .band_edits(body, source.module_range())
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The seating forecast over one body: its member blocks, the comments
/// the banding carries between members, the import bands it sorts, and
/// the body indices in the order the band seats them.
pub(crate) struct Bands {
    pub(crate) blocks: Vec<TextRange>,
    pub(crate) carries: Vec<Carry>,
    pub(crate) imports: Vec<ImportBand>,
    pub(crate) order: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// `rule`'s forecast over the whole module body of `source`.
    fn module_forecast(
        rule: &BandConstants,
        source: &Source,
        defer_annotations: bool,
    ) -> Option<Bands> {
        rule.forecast(
            source,
            &source.ast().body,
            source.module_range(),
            defer_annotations,
        )
    }

    #[rstest]
    #[case::deferred_annotation_frees_the_observed_site(true, vec![2, 1, 0])]
    #[case::eager_annotation_anchors_the_observed_site(false, vec![0, 1, 2])]
    fn forecast_reads_annotations_as_eager_unless_deferred(
        #[case] defer_annotations: bool,
        #[case] order: Vec<usize>,
    ) {
        let source =
            parse("def f(x: Foo.Bar):\n    pass\n\n\nY = Foo.Bar\n\n\nfrom m import Foo\n");
        let rule = BandConstants::from_config(&Config::default());
        let bands = module_forecast(&rule, &source, defer_annotations).expect("the body bands");
        assert_eq!(bands.order, order);
    }

    #[rstest]
    #[case::hoist_joins_the_runs("from p import a\nX = 1\nfrom q import b\n\nprint(a, b, X)\n", Some((vec![0, 2], 0)))]
    #[case::sort_reseats_the_head("from .p import a\nfrom ..q import b\n", Some((vec![0, 1], 1)))]
    #[case::pinned_anchor_splits_the_band("from p import a\nprint(a)\nfrom q import b\n", Some((vec![0], 0)))]
    #[case::shared_line_declines("from p import a; from q import b\n", None)]
    fn forecast_reads_the_band_the_hoist_seats(
        #[case] src: &str,
        #[case] first: Option<(Vec<usize>, usize)>,
    ) {
        let source = parse(src);
        let rule = BandConstants::from_config(&Config::default());
        let bands = module_forecast(&rule, &source, false);
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
    fn forecast_reads_the_comment_the_banding_carries(
        #[case] src: &str,
        #[case] first: Option<(usize, usize, bool)>,
    ) {
        let source = parse(src);
        let rule = BandConstants {
            code_width: 40,
            ..BandConstants::from_config(&Config::default())
        };
        let bands = module_forecast(&rule, &source, false);
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

    #[rstest]
    #[case::inert_constant_leads(
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = int\n",
        vec![1, 0]
    )]
    #[case::import_leads(
        "def convert(value: Sequence) -> Sequence:\n    return value\n\n\nfrom collections.abc import Sequence\n",
        vec![1, 0]
    )]
    #[case::effectful_constant_pins("def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = build()\n", vec![0, 1])]
    #[case::trailing_constant_declines_under_its_reader(
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = convert\n",
        vec![0, 1]
    )]
    fn forecast_seats_the_body_as_the_band_lays_it_out(
        #[case] src: &str,
        #[case] order: Vec<usize>,
    ) {
        let source = parse(src);
        let rule = BandConstants::from_config(&Config::default());
        let bands = module_forecast(&rule, &source, false).expect("the body bands");
        assert_eq!(bands.order, order);
    }
}

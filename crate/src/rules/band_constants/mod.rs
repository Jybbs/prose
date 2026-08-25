//! Module-scope constant banding. Hoists single-name assignments into a
//! leading band below the imports and a trailing band beneath the
//! definitions, pinning both ends of any eager reference the assembled
//! order would seat backward so the band forms around them, and
//! declining the body only when no participant is left to pin. The
//! rule walks the module body and each module-scope compound arm,
//! applying the [`plan`] analysis and emitting one fix group per banded
//! body, or one per cell over a notebook.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{PythonVersion, Stmt, helpers::is_compound_statement};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        comments::TRAILING_GAP,
        edit::splice_bodies,
        imports::defers_annotations,
        orderer::{
            any_sibling_shares_line, assemble_or_borrow, assembled_cell_edits, member_blocks,
            rendered_member_blocks,
        },
        scope::{compound_sub_bodies, scoped_body},
        sections::Sections,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod analysis;
mod plan;

pub(crate) use self::plan::{Carry, ImportBand};
use self::{
    analysis::module_band_plan,
    plan::{Banding, banded_gap},
};

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
        let order: Vec<usize> = (0..body.len()).collect();
        let (imports, carries) = module_band_plan(
            source,
            body,
            &blocks,
            self.code_width,
            defers_annotations(&source.ast().body),
            self.group_subcategories,
            self.target_version,
        )?
        .import_bands(
            body,
            &sections,
            &self.first_party,
            self.group_imports,
            &order,
        )?;
        Some(Bands {
            blocks,
            carries,
            imports,
        })
    }
}

impl Rule for BandConstants {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let bander = Bander {
            defer_annotations: defers_annotations(body),
            rule: self,
            source,
        };
        let layout = bander.band_layout(body, source.module_range());
        assembled_cell_edits(
            source,
            &layout.blocks,
            &layout.rendered,
            &layout.order,
            layout.forced(),
            |i| bander.band_gap(&layout, body, i),
        )
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

/// Invariant banding context threaded through the recursion.
struct Bander<'a> {
    defer_annotations: bool,
    rule: &'a BandConstants,
    source: &'a Source,
}

impl<'a> Bander<'a> {
    /// Bands a module-scope body, returning the rewritten text alongside
    /// the block-extent span it covers. Each member's text folds in any
    /// banded module-scope compound arm beneath it, so a banded arm splices
    /// into its parent member rather than emitting on its own. The text is
    /// `Cow::Owned` when the band reorders or a descendant arm rewrites,
    /// falling back to `Cow::Borrowed` over `source.slice(span)`.
    fn band_body(&self, body: &'a [Stmt], outer: TextRange) -> (Cow<'a, str>, TextRange) {
        let layout = self.band_layout(body, outer);
        assemble_or_borrow(
            self.source,
            &layout.blocks,
            &layout.rendered,
            &layout.order,
            layout.forced(),
            |i| self.band_gap(&layout, body, i),
        )
    }

    /// The divider [`banded_gap`] places after new-order slot `i` of
    /// `layout`, `None` when no band applies or the ranks abut with no gap.
    fn band_gap(&self, layout: &BandLayout<'_>, body: &[Stmt], i: usize) -> Option<&'static str> {
        layout.band.as_ref().and_then(|b| {
            banded_gap(
                b,
                body,
                &self.rule.first_party,
                self.rule.group_imports,
                self.source.line_ending(),
                layout.order[i],
                layout.order[i + 1],
            )
        })
    }

    /// Renders `body`, builds the module band over it, and moves each
    /// carried comment onto the member it binds to, leaving the assembly
    /// to the caller. The section partition walls each notebook cell, so
    /// a band never crosses one.
    fn band_layout(&self, body: &'a [Stmt], outer: TextRange) -> BandLayout<'a> {
        let (blocks, mut rendered) =
            rendered_member_blocks(self.source, body, outer, |stmt, block| {
                self.band_stmt(stmt, block)
            });
        let mut order: Vec<usize> = (0..body.len()).collect();
        let band = (!any_sibling_shares_line(self.source, body))
            .then(|| {
                let sections = Sections::of(self.source, &blocks);
                self.band_module_constants(body, &blocks, &sections, &mut order)
            })
            .flatten();
        if let Some(b) = &band {
            apply_band_comments(self.source, body, b, &mut rendered);
        }
        BandLayout {
            band,
            blocks,
            order,
            rendered,
        }
    }

    /// Builds the hoist plan over `body` and applies it to `order`,
    /// seating the leading band beneath the import run each section opens.
    /// Returns the [`Banding`] when the members relocated soundly.
    fn band_module_constants(
        &self,
        body: &'a [Stmt],
        blocks: &[TextRange],
        sections: &Sections,
        order: &mut Vec<usize>,
    ) -> Option<Banding> {
        let rule = self.rule;
        module_band_plan(
            self.source,
            body,
            blocks,
            rule.code_width,
            self.defer_annotations,
            rule.group_subcategories,
            rule.target_version,
        )?
        .apply(
            body,
            sections,
            &rule.first_party,
            rule.group_imports,
            rule.max_tiers,
            order,
        )
    }

    /// Folds a banded compound arm into `block`. A class or function
    /// definition leaves module scope, so its body holds no band and the
    /// block stays a borrow. A compound statement recurses into each arm
    /// with the inherited module scope. Any other statement is verbatim.
    fn band_stmt(&self, stmt: &'a Stmt, block: TextRange) -> Cow<'a, str> {
        if scoped_body(stmt).is_none() && is_compound_statement(stmt) {
            let bodies = compound_sub_bodies(stmt)
                .into_iter()
                .map(|(body, outer)| self.band_body(body, outer));
            return splice_bodies(self.source, block, bodies, &[]);
        }
        Cow::Borrowed(self.source.slice(block))
    }
}

/// The banding layout of a module body: its member blocks, their
/// rendered text, the new-order permutation, and the applied band. The
/// combined [`Bander::band_body`] and the per-cell notebook emit read it.
struct BandLayout<'a> {
    band: Option<Banding>,
    blocks: Vec<TextRange>,
    order: Vec<usize>,
    rendered: Vec<Cow<'a, str>>,
}

impl BandLayout<'_> {
    /// True when the band opens a tier blank, forcing an owned assembly
    /// so the spacing lands even when the order is already settled.
    fn forced(&self) -> bool {
        self.band.as_ref().is_some_and(Banding::stratifies)
    }
}

/// Settles every comment the band moves or re-seats. The first pass
/// closes the blank run under a comment run still heading its own
/// member, so a banded block re-reads with the attachment it was
/// assembled from rather than binding backward onto whichever member
/// the band seats above it. The second drops the comment and the blank
/// run beneath it from the text of the member whose block folded them
/// in, that block opening on the comment itself, and the third prepends
/// or trails it on the carrier's text.
fn apply_band_comments<'src>(
    source: &'src Source,
    body: &[Stmt],
    band: &Banding,
    rendered: &mut [Cow<'src, str>],
) {
    for (&idx, comment) in &band.attached {
        let newline = source.newline_str();
        let head = usize::from(comment.end() - comment.start());
        let own_line = usize::from(source.text().line_start(body[idx].start()) - comment.start());
        if own_line <= head + newline.len() {
            continue;
        }
        let text = std::mem::take(&mut rendered[idx]);
        rendered[idx] = Cow::Owned(format!("{}{newline}{}", &text[..head], &text[own_line..]));
    }
    for carry in &band.carries {
        let own_line = source.text().line_start(body[carry.absorbs].start());
        let held = usize::from(own_line - carry.comment.start());
        rendered[carry.absorbs] = match std::mem::take(&mut rendered[carry.absorbs]) {
            Cow::Borrowed(text) => Cow::Borrowed(&text[held..]),
            Cow::Owned(mut text) => Cow::Owned(text.split_off(held)),
        };
    }
    for carry in &band.carries {
        let comment = source.slice(carry.comment);
        let carried = &rendered[carry.carrier];
        rendered[carry.carrier] = Cow::Owned(if carry.trails {
            // The block reaches back to its line start, so its indent
            // belongs to a line of its own rather than to a trailing slot.
            format!("{carried}{TRAILING_GAP}{}", comment.trim_start())
        } else {
            format!("{comment}{}{carried}", source.newline_str())
        });
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{primitives::orderer::member_blocks, testing::parse};

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
        let rule = BandConstants::from_config(&Config::default());
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
            ..BandConstants::from_config(&Config::default())
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

    #[test]
    fn band_module_constants_hoists_an_import_below_a_definition() {
        let source =
            parse("def helper(value):\n    return value\n\n\nimport os\n\n\nCONFIG = helper\n");
        let body = &source.ast().body;
        let blocks = member_blocks(&source, body, source.module_range());
        let mut order: Vec<usize> = (0..body.len()).collect();
        let rule = BandConstants {
            code_width: 88,
            first_party: Vec::new(),
            group_imports: true,
            group_subcategories: true,
            max_tiers: Some(2),
            target_version: None,
        };
        let bander = Bander {
            defer_annotations: false,
            rule: &rule,
            source: &source,
        };
        let sections = Sections::of(&source, &blocks);
        bander
            .band_module_constants(body, &blocks, &sections, &mut order)
            .expect("a definition before an import bands without panicking");
        assert_eq!(
            order,
            vec![1, 0, 2],
            "the import hoists above the def and CONFIG pools below it",
        );
    }
}

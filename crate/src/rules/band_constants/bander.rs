//! The assembly `band-constants` drives over a module body: the
//! per-body layout the band resolves, the recursion splicing a banded
//! compound arm back into its parent member, and the comment moves the
//! banding settles on the rendered text.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, helpers::is_compound_statement};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use super::{
    BandConstants,
    plan::{Banding, banded_gap},
};
use crate::{
    primitives::{
        comments::TRAILING_GAP,
        orderer::{Assembly, any_sibling_shares_line, rendered_member_blocks},
        scope::{scoped_body, splice_compound_arms},
        sections::Sections,
    },
    source::Source,
};

/// Invariant banding context threaded through the recursion.
pub(super) struct Bander<'a> {
    pub(super) defer_annotations: bool,
    pub(super) rule: &'a BandConstants,
    pub(super) source: &'a Source,
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
        layout
            .assembly
            .or_borrow(self.source, layout.forced(), |i| {
                self.band_gap(&layout, body, i)
            })
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
                layout.assembly.order[i],
                layout.assembly.order[i + 1],
            )
        })
    }

    /// Renders `body`, builds the module band over it, and moves each
    /// carried comment onto the member it binds to, leaving the assembly
    /// to the caller. The section partition walls each notebook cell, so
    /// a band never crosses one.
    fn band_layout(&self, body: &'a [Stmt], outer: TextRange) -> BandLayout<'a> {
        let mut assembly = rendered_member_blocks(self.source, body, outer, |stmt, block| {
            self.band_stmt(stmt, block)
        });
        let band = (!any_sibling_shares_line(self.source, body))
            .then(|| {
                let sections = Sections::of(self.source, &assembly.blocks);
                self.band_module_constants(body, &assembly.blocks, &sections, &mut assembly.order)
            })
            .flatten();
        if let Some(b) = &band {
            apply_band_comments(self.source, body, b, &mut assembly.rendered);
        }
        BandLayout { assembly, band }
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
        rule.plan(self.source, body, blocks, self.defer_annotations)?
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
            return splice_compound_arms(self.source, stmt, block, &[], |body, outer| {
                self.band_body(body, outer)
            });
        }
        Cow::Borrowed(self.source.slice(block))
    }

    /// One fix group per banded body, or one per notebook cell,
    /// assembled from the layout the band settles over `body`.
    pub(super) fn band_edits(&self, body: &'a [Stmt], outer: TextRange) -> Vec<Vec<Edit>> {
        let layout = self.band_layout(body, outer);
        layout
            .assembly
            .cell_edits(self.source, layout.forced(), |i| {
                self.band_gap(&layout, body, i)
            })
    }
}

/// The banding layout of a module body, its assembly beside the band
/// applied over it. The combined [`Bander::band_body`] and the per-cell
/// [`Bander::band_edits`] read it.
struct BandLayout<'a> {
    assembly: Assembly<'a>,
    band: Option<Banding>,
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
    use crate::{config::Config, primitives::orderer::member_blocks, testing::parse};

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

    #[rstest]
    #[case("def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = int\n")]
    #[case(
        "from __future__ import annotations\n\nimport os\n\n\ndef f():\n    return os\n\n\nLIMIT = 1\n"
    )]
    #[case("X = 1\n\n# note\n\ndef f():\n    pass\n\n\nimport os\n")]
    fn forecast_order_matches_the_applied_layout(#[case] src: &str) {
        let source = parse(src);
        let body = &source.ast().body;
        let rule = BandConstants::from_config(&Config::default());
        let bander = Bander {
            defer_annotations: false,
            rule: &rule,
            source: &source,
        };
        let forecast = rule
            .forecast(&source, body, source.module_range(), false)
            .expect("the body bands");
        assert_eq!(
            bander
                .band_layout(body, source.module_range())
                .assembly
                .order,
            forecast.order
        );
    }
}

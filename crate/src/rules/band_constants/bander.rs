//! The body-assembly recursion behind `band-constants`. Each
//! module-scope compound arm bands within its parent member, the
//! module band's comments settle onto the members they bind to, and
//! the assembly emits one fix group per banded body, or one per cell
//! over a notebook.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, helpers::is_compound_statement};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use super::{
    BandConstants,
    analysis::module_band_plan,
    plan::{Banding, banded_gap},
};
use crate::{
    primitives::{
        comments::TRAILING_GAP,
        imports::defers_annotations,
        orderer::{Assembly, any_sibling_shares_line, rendered_member_blocks},
        scope::{scoped_body, splice_compound_arms},
        sections::Sections,
    },
    source::Source,
};

/// The banding layout of a module body: its assembly of member blocks,
/// rendered text, and new-order permutation, and the applied band. The
/// combined [`Bander::band_body`] and the per-cell notebook emit read it.
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
        layout.assembly.assemble(self.source, layout.forced(), |i| {
            self.band_gap(&layout, body, i)
        })
    }

    /// The divider [`banded_gap`] places after new-order slot `i` of
    /// `layout`, `None` when no band applies or the pair keeps its source
    /// gap.
    fn band_gap(&self, layout: &BandLayout<'_>, body: &[Stmt], i: usize) -> Option<&'static str> {
        layout.band.as_ref().and_then(|b| {
            banded_gap(
                b,
                body,
                self.rule,
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
            assembly: Assembly {
                blocks,
                order,
                rendered,
            },
            band,
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
        module_band_plan(self.source, body, blocks, self.rule, self.defer_annotations)?
            .apply(body, sections, self.rule, order)
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
}

/// The fix groups banding `source`'s module body under `rule`, one per
/// notebook cell the body spans and one for an ordinary module.
pub(super) fn band_module(rule: &BandConstants, source: &Source) -> Vec<Vec<Edit>> {
    let body = &source.ast().body;
    if body.is_empty() {
        return Vec::new();
    }
    let bander = Bander {
        defer_annotations: defers_annotations(body),
        rule,
        source,
    };
    let layout = bander.band_layout(body, source.module_range());
    layout.assembly.cell_edits(source, layout.forced(), |i| {
        bander.band_gap(&layout, body, i)
    })
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
    let newline = source.newline_str();
    for (&idx, comment) in &band.attached {
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
            format!("{comment}{newline}{carried}")
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives::orderer::member_blocks, testing::parse};

    #[test]
    fn band_module_constants_hoists_an_import_below_a_definition() {
        let source =
            parse("def helper(value):\n    return value\n\n\nimport os\n\n\nCONFIG = helper\n");
        let body = &source.ast().body;
        let blocks = member_blocks(&source, body, source.module_range());
        let mut order: Vec<usize> = (0..body.len()).collect();
        let rule = BandConstants::default();
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

//! Module-scope constant banding. Hoists single-name assignments into a
//! leading band below the imports and a trailing band beneath the
//! definitions, declining whenever the assembled order would seat an
//! eager reference ahead of its definition. The rule walks the module
//! body and each module-scope compound arm, applying the [`plan`]
//! analysis and emitting one edit per banded body, or one per cell over
//! a notebook.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{PythonVersion, Stmt, helpers::is_compound_statement};
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    primitives::{
        edit::{singleton_groups, splice_bodies},
        imports::defers_annotations,
        orderer::{
            any_sibling_shares_line, assemble_or_borrow, assembled_cell_edits,
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

use self::{
    analysis::module_band_plan,
    plan::{Banding, banded_gap},
};

pub(crate) struct BandConstants {
    first_party: Vec<String>,
    group_imports: bool,
    target_version: Option<PythonVersion>,
}

impl BandConstants {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            target_version: config.target_version,
        }
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
            first_party: &self.first_party,
            group_imports: self.group_imports,
            source,
            target_version: self.target_version,
        };
        let layout = bander.band_layout(body, source.module_range());
        let edits = assembled_cell_edits(
            source,
            &layout.blocks,
            &layout.rendered,
            &layout.order,
            false,
            |i| {
                layout.band.as_ref().and_then(|b| {
                    banded_gap(
                        &b.ranks,
                        body,
                        &self.first_party,
                        self.group_imports,
                        layout.order[i],
                        layout.order[i + 1],
                    )
                })
            },
        );
        singleton_groups(edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
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

/// Invariant banding context threaded through the recursion.
struct Bander<'a> {
    defer_annotations: bool,
    first_party: &'a [String],
    group_imports: bool,
    source: &'a Source,
    target_version: Option<PythonVersion>,
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
            false,
            |i| {
                layout.band.as_ref().and_then(|b| {
                    banded_gap(
                        &b.ranks,
                        body,
                        self.first_party,
                        self.group_imports,
                        layout.order[i],
                        layout.order[i + 1],
                    )
                })
            },
        )
    }

    /// Renders `body`, builds the module band over it, and folds each
    /// carried comment up with its constant, leaving the assembly to the
    /// caller. The section partition walls each notebook cell, so a band
    /// never crosses one.
    fn band_layout(&self, body: &'a [Stmt], outer: TextRange) -> BandLayout<'a> {
        let (mut blocks, mut rendered) =
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
            apply_band_carries(self.source, b, &mut blocks, &mut rendered);
        }
        BandLayout {
            band,
            blocks,
            order,
            rendered,
        }
    }

    /// Folds a banded compound arm into `block`. A class or function
    /// definition leaves module scope, so its body holds no band and the
    /// block stays a borrow. A compound statement recurses into each arm
    /// with the inherited module scope. Any other statement is verbatim.
    fn band_stmt(&self, stmt: &'a Stmt, block: TextRange) -> Cow<'a, str> {
        if scoped_body(stmt).is_none() && is_compound_statement(stmt) {
            let bodies = compound_sub_bodies(stmt)
                .into_iter()
                .filter(|(body, _)| !body.is_empty())
                .map(|(body, outer)| self.band_body(body, outer));
            return splice_bodies(self.source, block, bodies, &[]);
        }
        Cow::Borrowed(self.source.slice(block))
    }

    /// Builds the hoist plan over `body` and applies it to `order`,
    /// seating the leading band beneath the import run each section opens.
    /// Returns the [`Banding`] when constants relocated soundly.
    fn band_module_constants(
        &self,
        body: &'a [Stmt],
        blocks: &[TextRange],
        sections: &Sections,
        order: &mut Vec<usize>,
    ) -> Option<Banding> {
        module_band_plan(
            self.source,
            body,
            blocks,
            self.defer_annotations,
            self.target_version,
        )?
        .apply(body, sections, self.first_party, self.group_imports, order)
    }
}

/// Relocates each carried comment up with its banded constant, extending
/// the constant's block back over the comment and prepending it to the
/// rendered text so the hoist moves the comment rather than stranding it.
fn apply_band_carries(
    source: &Source,
    band: &Banding,
    blocks: &mut [TextRange],
    rendered: &mut [Cow<'_, str>],
) {
    for &(idx, comment) in &band.carries {
        let carried = format!(
            "{}{}{}",
            source.slice(comment),
            source.newline_str(),
            rendered[idx],
        );
        blocks[idx] = comment.cover(blocks[idx]);
        rendered[idx] = Cow::Owned(carried);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::orderer::block_ranges;
    use crate::testing::parse;

    #[test]
    fn band_module_constants_hoists_an_import_below_a_definition() {
        let source =
            parse("def helper(value):\n    return value\n\n\nimport os\n\n\nCONFIG = helper\n");
        let body = &source.ast().body;
        let blocks = block_ranges(&source, body, source.module_range());
        let mut order: Vec<usize> = (0..body.len()).collect();
        let bander = Bander {
            defer_annotations: false,
            first_party: &[],
            group_imports: true,
            source: &source,
            target_version: None,
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

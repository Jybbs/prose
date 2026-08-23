//! The band plan and its application. [`BandPlan::apply`] drains each
//! section's slots into imports, leading constants, definitions, then
//! trailing constants, declining when the assembled order would seat an
//! eager reference ahead of its definition. [`banded_gap`] decides the
//! blank between two seated bands.

use std::collections::HashMap;

use itertools::Itertools;
use ruff_python_ast::Stmt;
use ruff_text_size::TextRange;

use crate::primitives::{
    blanks::{blank_gap, module_blank_lines},
    imports::{import_blank_lines, import_sort_key},
    sections::Sections,
    slots::slot_positions,
};

/// The applied banding: a band rank per banded statement, the rendered
/// tier each banded constant sits in, the member count per rendered
/// tier, the comment run still heading each member, and the comment each
/// member carries onto another member's line.
pub(super) struct Banding {
    pub(super) attached: HashMap<usize, TextRange>,
    pub(super) carries: Vec<Carry>,
    ranks: HashMap<usize, BandRank>,
    tier_sizes: HashMap<(BandRank, usize), usize>,
    tiers: HashMap<usize, usize>,
}

impl Banding {
    /// True when `idx` opens a blank-separated sub-band, meaning its
    /// rendered tier climbs past the base and holds at least two
    /// members. A lone nested constant folds tight into the tier above
    /// and aligns with it.
    fn opens_band(&self, idx: usize) -> bool {
        let tier = self.rendered_tier(idx);
        tier > 0
            && self
                .tier_sizes
                .get(&(self.ranks[&idx], tier))
                .is_some_and(|&members| members >= 2)
    }

    /// The rendered tier `idx` sits in, the true tier already clamped
    /// under `max_tiers` at build so a capped band folds its deeper tiers
    /// into the last. A member outside the band renders at the base tier.
    fn rendered_tier(&self, idx: usize) -> usize {
        self.tiers.get(&idx).copied().unwrap_or(0)
    }

    /// True when any banded constant opens a blank-separated sub-band, so
    /// the assembly re-emits even when the order is already settled.
    pub(super) fn stratifies(&self) -> bool {
        self.tiers.keys().any(|&idx| self.opens_band(idx))
    }
}

/// The module-scope hoist plan: a band rank per banded statement, the
/// intra-band `(tier, subcategory, name)` key per banded constant, the
/// eager-reference edges the order keeps backward, the comment run each
/// member's block folds in ahead of its code, and the comment each
/// carries onto another member's line. A statement absent from `ranks`
/// is a pinned anchor.
pub(super) struct BandPlan<'src> {
    pub(super) attached: HashMap<usize, TextRange>,
    pub(super) carries: Vec<Carry>,
    pub(super) edges: Vec<(usize, usize)>,
    pub(super) keys: HashMap<usize, (usize, Subcategory, &'src str)>,
    pub(super) ranks: HashMap<usize, BandRank>,
}

impl BandPlan<'_> {
    /// Appends `region`'s body indices to the drained order, the import
    /// run sorted to the front, the leading constants below it, the
    /// definitions in incoming order, the trailing constants last. The
    /// import run sorts by group then name when `grouped`, flat otherwise,
    /// and is recorded as one import band. Both constant
    /// bands sort by `(tier, subcategory, name)`. Records a `(from, to)`
    /// shift for every sorted band whose head member changed. Clears
    /// `region`.
    fn drain_region(
        &self,
        body: &[Stmt],
        first_party: &[String],
        grouped: bool,
        region: &mut Vec<usize>,
        drained: &mut Drained,
    ) {
        let mut imports = Vec::new();
        let mut leading = Vec::new();
        let mut definitions = Vec::new();
        let mut trailing = Vec::new();
        for idx in region.drain(..) {
            match self.ranks[&idx] {
                BandRank::Import => imports.push(idx),
                BandRank::Leading => leading.push(idx),
                BandRank::Definition => definitions.push(idx),
                BandRank::Trailing => trailing.push(idx),
            }
        }
        let heads = |bands: [&[usize]; 3]| bands.map(|band| band.first().copied());
        let source_heads = heads([&imports, &leading, &trailing]);
        let slots = imports.clone();
        imports.sort_by_key(|&idx| {
            import_sort_key(&body[idx], first_party, grouped)
                .expect("import band holds only imports")
        });
        if let Some(&sorted_head) = imports.first() {
            drained.imports.push(ImportBand { slots, sorted_head });
        }
        leading.sort_by_key(|idx| self.keys[idx]);
        trailing.sort_by_key(|idx| self.keys[idx]);
        let sorted_heads = heads([&imports, &leading, &trailing]);
        drained.shifts.extend(
            source_heads
                .into_iter()
                .zip(sorted_heads)
                .filter_map(|(before, after)| before.zip(after))
                .filter(|(before, after)| before != after),
        );
        let banded = &mut drained.banded;
        banded.append(&mut imports);
        banded.append(&mut leading);
        banded.append(&mut definitions);
        banded.append(&mut trailing);
    }

    /// Drains `order` into the banded order section by section, a
    /// section marker and a pinned anchor each closing the running
    /// region, `None` when an eager reference would seat ahead of its
    /// definition.
    fn drained(
        &self,
        body: &[Stmt],
        sections: &Sections,
        first_party: &[String],
        grouped: bool,
        order: &[usize],
    ) -> Option<Drained> {
        let mut drained = Drained {
            banded: Vec::with_capacity(order.len()),
            imports: Vec::new(),
            shifts: Vec::new(),
        };
        let mut region = Vec::new();
        for (slot, &idx) in order.iter().enumerate() {
            if sections.is_boundary(slot) {
                self.drain_region(body, first_party, grouped, &mut region, &mut drained);
            }
            if self.ranks.contains_key(&idx) {
                region.push(idx);
            } else {
                self.drain_region(body, first_party, grouped, &mut region, &mut drained);
                drained.banded.push(idx);
            }
        }
        self.drain_region(body, first_party, grouped, &mut region, &mut drained);
        self.is_sound(&drained.banded).then_some(drained)
    }

    /// Every import band the drained `order` seats, `None` when the plan
    /// is unsound.
    pub(super) fn import_bands(
        &self,
        body: &[Stmt],
        sections: &Sections,
        first_party: &[String],
        grouped: bool,
        order: &[usize],
    ) -> Option<Vec<ImportBand>> {
        self.drained(body, sections, first_party, grouped, order)
            .map(|drained| drained.imports)
    }

    /// True when every eager reference seats its referent ahead of the
    /// referrer in `order`, the import-safety invariant the hoist holds.
    fn is_sound(&self, order: &[usize]) -> bool {
        let position = slot_positions(order);
        self.edges
            .iter()
            .all(|&(from, to)| position[to] < position[from])
    }

    /// Moves each comment heading a band's source-order head onto the
    /// member the sort seated first, dropping it from the run heading
    /// its own member.
    fn relocate_heads(&mut self, shifts: &[(usize, usize)]) {
        for &(from, to) in shifts {
            if let Some(comment) = self.attached.remove(&from) {
                self.carries.push(Carry {
                    absorbs: from,
                    carrier: to,
                    comment,
                    trails: false,
                });
            }
        }
    }

    /// Applies the plan to `order`, draining each section's slots into
    /// imports, leading constants, definitions, then trailing constants.
    /// A comment heading a band's source-order head moves to whichever
    /// member the sort seats first, so it heads the band still. Returns
    /// the [`Banding`] when the plan is sound and the assembled order
    /// differs from `order`, moves a comment onto another member, or opens
    /// a tier blank line, rewriting `order` in place. Leaves `order`
    /// untouched otherwise.
    pub(super) fn apply(
        mut self,
        body: &[Stmt],
        sections: &Sections,
        first_party: &[String],
        grouped: bool,
        max_tiers: Option<usize>,
        order: &mut Vec<usize>,
    ) -> Option<Banding> {
        let Drained { banded, shifts, .. } =
            self.drained(body, sections, first_party, grouped, order)?;
        self.relocate_heads(&shifts);
        let tiers: HashMap<usize, usize> = self
            .keys
            .iter()
            .map(|(&idx, &(tier, ..))| {
                (
                    idx,
                    max_tiers.map_or(tier, |cap| tier.min(cap.saturating_sub(1))),
                )
            })
            .collect();
        let tier_sizes = tiers
            .iter()
            .counts_by(|(&idx, &tier)| (self.ranks[&idx], tier));
        let banding = Banding {
            attached: self.attached,
            carries: self.carries,
            ranks: self.ranks,
            tier_sizes,
            tiers,
        };
        (banded != *order || !banding.carries.is_empty() || banding.stratifies()).then(|| {
            *order = banded;
            banding
        })
    }
}

/// The band a statement hoists into. `drain_region` seats the bands as
/// imports, leading constants, definitions, then trailing constants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum BandRank {
    Definition,
    Import,
    Leading,
    Trailing,
}

/// A comment moving from the member whose block extent holds it onto
/// another member's rendered text, landing after that member's code when
/// `trails` and on the line above it otherwise. The block a comment
/// binds backward from and the band head a sort reseats are the two
/// moves, so `absorbs` and `carrier` always name different members.
pub(super) struct Carry {
    pub(super) absorbs: usize,
    pub(super) carrier: usize,
    pub(super) comment: TextRange,
    pub(super) trails: bool,
}

/// One import band: its body slots in source order and the slot the
/// sort seats first, whose line the band's heading then reads over.
pub(crate) struct ImportBand {
    pub(crate) slots: Vec<usize>,
    pub(crate) sorted_head: usize,
}

/// The kind a banded constant sorts into within its tier. A band keys on
/// `(tier, subcategory, name)`, so the aliases cluster ahead of the
/// constants, and the constants ahead of the remaining module state.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Subcategory {
    #[default]
    Alias,
    Constant,
    State,
}

/// The drained order of a body, its import bands, and the `(from, to)`
/// head shift of every band the sort reseated.
struct Drained {
    banded: Vec<usize>,
    imports: Vec<ImportBand>,
    shifts: Vec<(usize, usize)>,
}

/// The gap the banded order seats after the block of rank `a`, ahead of
/// the block of rank `b`. A same-band pair opens one blank line across a
/// tier boundary into a sub-band of two or more members, a lone nested
/// constant folding tight into the tier above instead, and an import run
/// keeps one blank line between canonical groups. Every other pair takes
/// the count [`module_blank_lines`] declares, one blank line standing in
/// wherever that policy holds no opinion. `None` falls back to the source
/// gap, the case for a pinned anchor on either side.
pub(super) fn banded_gap(
    band: &Banding,
    body: &[Stmt],
    first_party: &[String],
    grouped: bool,
    a: usize,
    b: usize,
) -> Option<&'static str> {
    let blanks = match (*band.ranks.get(&a)?, *band.ranks.get(&b)?) {
        (BandRank::Leading, BandRank::Leading) | (BandRank::Trailing, BandRank::Trailing) => {
            u32::from(band.rendered_tier(a) != band.rendered_tier(b) && band.opens_band(b))
        }
        (BandRank::Import, BandRank::Import) => {
            import_blank_lines(&body[a], &body[b], first_party, grouped).unwrap_or(1)
        }
        _ => module_blank_lines(&body[a], &body[b], first_party, grouped).unwrap_or(1),
    };
    Some(blank_gap(blanks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        primitives::orderer::member_blocks, rules::band_constants::analysis::module_band_plan,
        source::Source, testing::parse,
    };

    /// The banding `source` produces alongside the order it rewrote.
    fn banded(source: &Source) -> (Banding, Vec<usize>) {
        let body = &source.ast().body;
        let blocks = member_blocks(source, body, source.module_range());
        let sections = Sections::of(source, &blocks);
        let mut order: Vec<usize> = (0..body.len()).collect();
        let banding = module_band_plan(source, body, &blocks, 88, false, true, None)
            .expect("acyclic module plans")
            .apply(body, &sections, &[], true, None, &mut order)
            .expect("the band applies");
        (banding, order)
    }

    #[test]
    fn apply_leaves_a_backward_carry_on_its_own_member() {
        let source = parse("ZETA = 1\n# documents ZETA\n\nALPHA = 2\n");
        let (banding, order) = banded(&source);
        assert_eq!(order, vec![1, 0]);
        let carry = banding.carries.first().expect("ZETA carries its comment");
        assert_eq!(
            carry.carrier, 0,
            "the note stays with the member it documents"
        );
        assert!(carry.trails, "the joined line fits inside the budget");
    }

    #[test]
    fn apply_moves_a_heading_onto_the_reseated_band_head() {
        let source = parse("# the tunable knobs\nZETA = 1\nALPHA = 2\n");
        let (banding, order) = banded(&source);
        assert_eq!(order, vec![1, 0]);
        let carry = banding.carries.first().expect("the heading relocates");
        assert_eq!(carry.absorbs, 0, "ZETA's block still covers the comment");
        assert_eq!(carry.carrier, 1, "ALPHA heads the band after the sort");
        assert!(!carry.trails, "a heading opens the band on its own line");
    }
}

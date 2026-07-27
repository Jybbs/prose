//! The band plan and its application. [`BandPlan::apply`] drains each
//! section's slots into imports, leading constants, definitions, then
//! trailing constants, declining when the assembled order would seat an
//! eager reference ahead of its definition. [`banded_gap`] decides the
//! blank between two seated bands.

use std::collections::HashMap;

use ruff_python_ast::Stmt;
use ruff_text_size::TextRange;

use crate::primitives::{
    imports::{import_blank_lines, import_sort_key},
    orderer::slot_positions,
    sections::Sections,
};

/// The applied banding: a band rank per banded statement, the rendered
/// tier each banded constant sits in, the member count per rendered
/// tier, and the prose comment each member carries through the sort.
pub(super) struct Banding {
    pub(super) carries: Vec<Carry>,
    ranks: HashMap<usize, BandRank>,
    tier_sizes: HashMap<(BandRank, usize), usize>,
    tiers: HashMap<usize, usize>,
}

impl Banding {
    /// True when `idx` opens a blank-separated sub-band: its rendered
    /// tier climbs past the base and holds at least two members. A lone
    /// nested constant folds tight into the tier above and aligns with it.
    fn opens_band(&self, idx: usize) -> bool {
        let tier = self.rendered_tier(idx);
        let members = self
            .tier_sizes
            .get(&(self.ranks[&idx], tier))
            .copied()
            .unwrap_or(0);
        tier > 0 && members >= 2
    }

    /// True when a bound comment crosses its member's code to reach the
    /// side that keeps them bound, so the assembly re-emits even when the
    /// order is already settled.
    fn rebinds(&self) -> bool {
        self.carries
            .iter()
            .any(|carry| carry.placement != Placement::Above)
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
/// eager-reference edges the order keeps backward, the comment each
/// carries out of the surrounding gap, and the comment block already
/// folded into a member's own extent. A statement absent from `ranks`
/// is a pinned anchor.
pub(super) struct BandPlan<'src> {
    pub(super) attached: HashMap<usize, TextRange>,
    pub(super) carries: Vec<Carry>,
    pub(super) edges: Vec<(usize, usize)>,
    pub(super) keys: HashMap<usize, (usize, Subcategory, &'src str)>,
    pub(super) ranks: HashMap<usize, BandRank>,
}

impl BandPlan<'_> {
    /// Appends `region`'s body indices to `out`, the import run sorted to
    /// the front, the leading constants below it, the definitions in
    /// incoming order, the trailing constants last. The import run sorts by
    /// group then name when `grouped`, flat otherwise. Both constant bands
    /// sort by `(tier, subcategory, name)`. Pushes a `(from, to)` pair onto
    /// `shifts` for every sorted band whose head member changed. Clears
    /// `region`.
    fn drain_region(
        &self,
        body: &[Stmt],
        first_party: &[String],
        grouped: bool,
        region: &mut Vec<usize>,
        shifts: &mut Vec<(usize, usize)>,
        out: &mut Vec<usize>,
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
        imports.sort_by_key(|&idx| {
            import_sort_key(&body[idx], first_party, grouped)
                .expect("import band holds only imports")
        });
        leading.sort_by_key(|idx| self.keys[idx]);
        trailing.sort_by_key(|idx| self.keys[idx]);
        let sorted_heads = heads([&imports, &leading, &trailing]);
        shifts.extend(
            source_heads
                .into_iter()
                .zip(sorted_heads)
                .filter_map(|(before, after)| {
                    let (before, after) = (before?, after?);
                    (before != after).then_some((before, after))
                }),
        );
        out.append(&mut imports);
        out.append(&mut leading);
        out.append(&mut definitions);
        out.append(&mut trailing);
    }

    /// True when every eager reference seats its referent ahead of the
    /// referrer in `order`, the import-safety invariant the hoist holds.
    fn is_sound(&self, order: &[usize]) -> bool {
        let position = slot_positions(order);
        self.edges
            .iter()
            .all(|&(from, to)| position[to] < position[from])
    }

    /// Moves each comment bound to a band's source-order head onto the
    /// member the sort seated first.
    fn relocate_heads(&mut self, shifts: &[(usize, usize)]) {
        for &(from, to) in shifts {
            for carry in &mut self.carries {
                if carry.carrier == from && carry.placement == Placement::Above {
                    carry.carrier = to;
                }
            }
            if let Some(&comment) = self.attached.get(&from) {
                self.carries.push(Carry {
                    absorbs: from,
                    carrier: to,
                    comment,
                    placement: Placement::Above,
                });
            }
        }
    }

    /// Applies the plan to `order`, draining each section's slots into
    /// imports, leading constants, definitions, then trailing constants.
    /// A section marker drains the running region, so a band never crosses
    /// a divider. A comment bound to a band's source-order head moves to
    /// whichever member the sort seats first, so it heads the band still.
    /// Returns the [`Banding`] when the plan is sound and the assembled
    /// order either differs from `order` or opens a tier blank line,
    /// rewriting `order` in place. Leaves `order` untouched otherwise.
    pub(super) fn apply(
        mut self,
        body: &[Stmt],
        sections: &Sections,
        first_party: &[String],
        grouped: bool,
        max_tiers: Option<usize>,
        order: &mut Vec<usize>,
    ) -> Option<Banding> {
        let mut shifts = Vec::new();
        let mut banded = Vec::with_capacity(order.len());
        let mut region = Vec::new();
        let mut drain = |region: &mut Vec<usize>, banded: &mut Vec<usize>| {
            self.drain_region(body, first_party, grouped, region, &mut shifts, banded);
        };
        for (slot, &idx) in order.iter().enumerate() {
            if sections.is_boundary(slot) {
                drain(&mut region, &mut banded);
            }
            if self.ranks.contains_key(&idx) {
                region.push(idx);
            } else {
                drain(&mut region, &mut banded);
                banded.push(idx);
            }
        }
        drain(&mut region, &mut banded);
        if !self.is_sound(&banded) {
            return None;
        }
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
        let mut tier_sizes: HashMap<(BandRank, usize), usize> = HashMap::new();
        for (&idx, &tier) in &tiers {
            *tier_sizes.entry((self.ranks[&idx], tier)).or_default() += 1;
        }
        let banding = Banding {
            carries: self.carries,
            ranks: self.ranks,
            tier_sizes,
            tiers,
        };
        (banded != *order || banding.rebinds() || banding.stratifies()).then(|| {
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

/// A comment bound to a banded member: the member whose block extent
/// absorbs it out of the surrounding gap, the member whose rendered text
/// carries it, and where the assembly seats it. The two members differ
/// only where the comment heads a band the sort reseats.
pub(super) struct Carry {
    pub(super) absorbs: usize,
    pub(super) carrier: usize,
    pub(super) comment: TextRange,
    pub(super) placement: Placement,
}

impl Carry {
    /// A comment the source placed above `idx`, which renders there still.
    pub(super) fn above(idx: usize, comment: TextRange) -> Self {
        Self {
            absorbs: idx,
            carrier: idx,
            comment,
            placement: Placement::Above,
        }
    }

    /// A comment the source placed below `idx`, landing after that
    /// member's code when `trails` and on the line above it otherwise.
    pub(super) fn below(idx: usize, comment: TextRange, trails: bool) -> Self {
        Self {
            absorbs: idx,
            carrier: idx,
            comment,
            placement: if trails {
                Placement::Trails
            } else {
                Placement::Climbs
            },
        }
    }
}

/// Where the assembly seats a bound comment. One the source placed below
/// its member either trails that member's code or climbs above it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Placement {
    Above,
    Climbs,
    Trails,
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

/// The gap the banded order seats after the block of rank `a`, ahead of
/// the block of rank `b`. A same-band pair opens one blank line across a
/// tier boundary into a sub-band of two or more members, a lone nested
/// constant folding tight into the tier above instead, a definition
/// fronts on two blank lines, and an import run keeps one blank line
/// between canonical groups. `None` falls back to the source gap, the
/// case for a pinned anchor on either side, leaving its spacing intact.
pub(super) fn banded_gap(
    band: &Banding,
    body: &[Stmt],
    first_party: &[String],
    grouped: bool,
    a: usize,
    b: usize,
) -> Option<&'static str> {
    Some(match (*band.ranks.get(&a)?, *band.ranks.get(&b)?) {
        (BandRank::Leading, BandRank::Leading) | (BandRank::Trailing, BandRank::Trailing) => {
            if band.rendered_tier(a) != band.rendered_tier(b) && band.opens_band(b) {
                "\n\n"
            } else {
                "\n"
            }
        }
        (BandRank::Import, BandRank::Import)
            if import_blank_lines(&body[a], &body[b], first_party, grouped) == Some(0) =>
        {
            "\n"
        }
        (_, BandRank::Definition) | (BandRank::Definition, _) => "\n\n\n",
        _ => "\n\n",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::orderer::member_blocks;
    use crate::rules::band_constants::analysis::module_band_plan;
    use crate::source::Source;
    use crate::testing::parse;

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
        assert_eq!(carry.placement, Placement::Trails);
    }

    #[test]
    fn apply_retargets_an_above_carry_to_the_reseated_band_head() {
        let source = parse("# the tunable knobs\nZETA = 1\nALPHA = 2\n");
        let (banding, order) = banded(&source);
        assert_eq!(order, vec![1, 0]);
        let carry = banding.carries.first().expect("the heading relocates");
        assert_eq!(carry.absorbs, 0, "ZETA's block still covers the comment");
        assert_eq!(carry.carrier, 1, "ALPHA heads the band after the sort");
        assert_eq!(carry.placement, Placement::Above);
    }
}

//! The band plan and its application. [`BandPlan::apply`] drains each
//! section's slots into imports, leading constants, definitions, then
//! trailing constants, declining when the assembled order would seat an
//! eager reference ahead of its definition. [`banded_gap`] decides the
//! blank between two seated bands.

use itertools::Itertools;
use ruff_python_ast::Stmt;
use ruff_source_file::LineEnding;
use ruff_text_size::TextRange;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

use super::{BandConstants, Bands};
use crate::primitives::{
    blanks::{blank_gap, module_blank_lines},
    group_map,
    imports::{import_blank_lines, import_sort_key},
    sections::Sections,
};

/// The applied banding: a band rank per banded statement, the constants
/// the reference analysis pinned and those of them a heading comment
/// leads with a blank line on each side of it, the rendered tier each
/// banded constant sits in, the member count per rendered tier, the
/// comment run still heading each member, and the comment each member
/// carries onto another member's line.
pub(super) struct Banding {
    anchored: FxHashSet<usize>,
    pub(super) attached: FxHashMap<usize, TextRange>,
    pub(super) carries: Vec<Carry>,
    detached: FxHashSet<usize>,
    ranks: FxHashMap<usize, BandRank>,
    tier_sizes: FxHashMap<(BandRank, usize), usize>,
    tiers: FxHashMap<usize, usize>,
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

    /// The rank `idx` spaces by against `beside`. A banded member holds
    /// its own rank. An anchored constant takes the rank of the constant
    /// band `beside` sits in, and the leading rank beside an import, a
    /// definition, or another anchored constant, where only its standing
    /// as a constant reaches the decision. `None` for every other pinned
    /// statement, whose pair keeps the source gap.
    fn spacing_rank(&self, idx: usize, beside: usize) -> Option<BandRank> {
        self.ranks.get(&idx).copied().or_else(|| {
            self.anchored
                .contains(&idx)
                .then(|| match self.ranks.get(&beside) {
                    Some(BandRank::Trailing) => BandRank::Trailing,
                    _ => BandRank::Leading,
                })
        })
    }

    /// True when any banded constant opens a blank-separated sub-band, so
    /// the assembly re-emits even when the order is already settled.
    pub(super) fn stratifies(&self) -> bool {
        self.tiers.keys().any(|&idx| self.opens_band(idx))
    }
}

/// The module-scope hoist plan: a band rank per banded statement, the
/// intra-band `(tier, subcategory, name)` key per banded constant, the
/// eager-reference edges the order keeps backward, each flagged where
/// its referent rebinds a name bound before the module body runs, the
/// comment run each
/// member's block folds in ahead of its code, and the comment each
/// carries onto another member's line. A statement absent from `ranks`
/// is a pinned anchor.
pub(super) struct BandPlan<'src> {
    pub(super) anchored: FxHashSet<usize>,
    pub(super) attached: FxHashMap<usize, TextRange>,
    pub(super) carries: Vec<Carry>,
    pub(super) detached: FxHashSet<usize>,
    pub(super) edges: Vec<(usize, usize, bool)>,
    pub(super) keys: FxHashMap<usize, (usize, Subcategory, &'src str)>,
    pub(super) ranks: FxHashMap<usize, BandRank>,
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
        rule: &BandConstants,
        region: &mut Vec<usize>,
        drained: &mut Drained,
    ) {
        let incoming = std::mem::take(region);
        if incoming.is_empty() {
            return;
        }
        let mut bands = group_map(incoming.iter().map(|&idx| (self.ranks[&idx], idx)));
        let mut take = |rank| bands.remove(&rank).unwrap_or_default();
        let mut imports = take(BandRank::Import);
        let mut leading = take(BandRank::Leading);
        let definitions = take(BandRank::Definition);
        let mut trailing = take(BandRank::Trailing);
        let heads = |bands: [&[usize]; 3]| bands.map(|band| band.first().copied());
        let source_heads = heads([&imports, &leading, &trailing]);
        let slots = imports.clone();
        imports.sort_by_key(|&idx| {
            import_sort_key(&body[idx], &rule.first_party, rule.group_imports)
                .expect("import band holds only imports")
        });
        leading.sort_by_key(|idx| self.keys[idx]);
        trailing.sort_by_key(|idx| self.keys[idx]);
        let banded = [imports.as_slice(), &leading, &definitions, &trailing].concat();
        let holds = self.region_holds_its_references(&banded);
        if let Some(&sorted_head) = if holds {
            imports.first()
        } else {
            slots.first()
        } {
            drained.imports.push(ImportBand { slots, sorted_head });
        }
        if !holds {
            drained.banded.extend(incoming);
            return;
        }
        let sorted_heads = heads([&imports, &leading, &trailing]);
        drained.shifts.extend(
            source_heads
                .into_iter()
                .zip(sorted_heads)
                .filter_map(|(before, after)| before.zip(after))
                .filter(|(before, after)| before != after),
        );
        drained.banded.extend(banded);
    }

    /// Drains `order` into the banded order section by section, a
    /// section marker and a pinned anchor each closing the running
    /// region, a region whose reorder would seat an eager reference
    /// ahead of its referent draining in its incoming order instead,
    /// and moves each comment heading a band's source-order head onto
    /// the member the sort seated first.
    fn drained(
        &mut self,
        body: &[Stmt],
        sections: &Sections,
        rule: &BandConstants,
        order: &[usize],
    ) -> Drained {
        let mut drained = Drained {
            banded: Vec::with_capacity(order.len()),
            imports: Vec::new(),
            shifts: Vec::new(),
        };
        let mut region = Vec::new();
        for (slot, &idx) in order.iter().enumerate() {
            if sections.is_boundary(slot) {
                self.drain_region(body, rule, &mut region, &mut drained);
            }
            if self.ranks.contains_key(&idx) {
                region.push(idx);
            } else {
                self.drain_region(body, rule, &mut region, &mut drained);
                drained.banded.push(idx);
            }
        }
        self.drain_region(body, rule, &mut region, &mut drained);
        self.relocate_heads(&drained.shifts);
        drained
    }

    /// True when `banded` seats every eager reference behind its
    /// referent, and keeps a reference to a name bound before the body
    /// runs on the side the source seated it on, a reader written above
    /// such a rebind staying above it and one written below staying
    /// below. Every other name is unbound until its statement runs, so
    /// hoisting it above a reader only ever resolves a reference. An
    /// edge reaching outside the region imposes nothing.
    fn region_holds_its_references(&self, banded: &[usize]) -> bool {
        let seat: FxHashMap<usize, usize> = banded
            .iter()
            .enumerate()
            .map(|(seat, &idx)| (idx, seat))
            .collect();
        self.edges.iter().all(|&(referrer, referent, prebound)| {
            match (seat.get(&referrer), seat.get(&referent)) {
                (Some(&seated_referrer), Some(&seated_referent)) if prebound => {
                    (referent < referrer) == (seated_referent < seated_referrer)
                }
                (Some(&seated_referrer), Some(&seated_referent)) => {
                    seated_referent < seated_referrer
                }
                _ => true,
            }
        })
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
        rule: &BandConstants,
        order: &mut Vec<usize>,
    ) -> Option<Banding> {
        let Drained { banded, .. } = self.drained(body, sections, rule, order);
        let tiers: FxHashMap<usize, usize> = self
            .keys
            .iter()
            .map(|(&idx, &(tier, ..))| (idx, rule.max_tiers.map_or(tier, |cap| tier.min(cap - 1))))
            .collect();
        let tier_sizes = tiers
            .iter()
            .counts_by_with_hasher(|(&idx, &tier)| (self.ranks[&idx], tier), FxBuildHasher);
        let banding = Banding {
            anchored: self.anchored,
            attached: self.attached,
            carries: self.carries,
            detached: self.detached,
            ranks: self.ranks,
            tier_sizes,
            tiers,
        };
        (banded != *order || !banding.carries.is_empty() || banding.stratifies()).then(|| {
            *order = banded;
            banding
        })
    }

    /// The seating forecast over `body`: `blocks`, the comments the
    /// banding carries onto another member, every import band it seats,
    /// and the order the band seats `body` in.
    pub(super) fn forecast(
        mut self,
        body: &[Stmt],
        blocks: Vec<TextRange>,
        sections: &Sections,
        rule: &BandConstants,
        order: &[usize],
    ) -> Bands {
        let Drained {
            banded, imports, ..
        } = self.drained(body, sections, rule, order);
        Bands {
            blocks,
            carries: self.carries,
            imports,
            order: banded,
        }
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
pub(crate) struct Carry {
    pub(crate) absorbs: usize,
    pub(crate) carrier: usize,
    pub(crate) comment: TextRange,
    pub(crate) trails: bool,
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
/// the block of rank `b`, an anchored constant on either side reading as
/// a member of the constant band beside it. A same-band pair opens one
/// blank line at a tier boundary whose higher tier is a sub-band of two
/// or more members, a lone nested constant folding tight into the tier
/// above instead, and one above an anchored constant a detached heading
/// leads. An import run keeps one blank line between canonical groups.
/// Every other pair takes the count [`module_blank_lines`] declares, one
/// blank line standing in wherever that policy holds no opinion,
/// rendered in `ending`. `None` falls back to the source gap, the case
/// for a pinned statement outside the constant analysis on either side.
pub(super) fn banded_gap(
    band: &Banding,
    body: &[Stmt],
    rule: &BandConstants,
    ending: LineEnding,
    a: usize,
    b: usize,
) -> Option<&'static str> {
    let blanks = match (band.spacing_rank(a, b)?, band.spacing_rank(b, a)?) {
        (BandRank::Leading, BandRank::Leading) | (BandRank::Trailing, BandRank::Trailing) => {
            let (tier_a, tier_b) = (band.rendered_tier(a), band.rendered_tier(b));
            u32::from(
                band.detached.contains(&b)
                    || (tier_a != tier_b && band.opens_band(if tier_a < tier_b { b } else { a })),
            )
        }
        (BandRank::Import, BandRank::Import) => {
            import_blank_lines(&body[a], &body[b], &rule.first_party, rule.group_imports)
                .unwrap_or(1)
        }
        _ => module_blank_lines(&body[a], &body[b], &rule.first_party, rule.group_imports)
            .unwrap_or(1),
    };
    Some(blank_gap(ending, blanks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, primitives::orderer::member_blocks,
        rules::band_constants::analysis::module_band_plan, source::Source, testing::parse,
    };

    /// The banding `source` produces alongside the order it rewrote.
    fn banded(source: &Source) -> (Banding, Vec<usize>) {
        let body = &source.ast().body;
        let blocks = member_blocks(source, body, source.module_range());
        let sections = Sections::of(source, &blocks);
        let mut order: Vec<usize> = (0..body.len()).collect();
        let rule = BandConstants {
            max_tiers: None,
            ..BandConstants::from_config(&Config::default())
        };
        let banding = module_band_plan(source, body, &blocks, &rule, false)
            .expect("acyclic module plans")
            .apply(body, &sections, &rule, &mut order)
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

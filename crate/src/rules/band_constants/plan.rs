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

/// The applied banding: a band rank per banded statement, and the prose
/// comment each banded constant carries up with it.
pub(super) struct Banding {
    pub(super) carries: Vec<(usize, TextRange)>,
    pub(super) ranks: HashMap<usize, BandRank>,
}

/// The module-scope hoist plan: a band rank per banded statement, the
/// intra-band `(tier, name)` key per banded constant, the eager-reference
/// edges the order keeps backward, and the comment each carries. A
/// statement absent from `ranks` is a pinned anchor.
pub(super) struct BandPlan<'src> {
    pub(super) carries: Vec<(usize, TextRange)>,
    pub(super) edges: Vec<(usize, usize)>,
    pub(super) keys: HashMap<usize, (usize, &'src str)>,
    pub(super) ranks: HashMap<usize, BandRank>,
}

impl BandPlan<'_> {
    /// Applies the plan to `order`, draining each section's slots into
    /// imports, leading constants, definitions, then trailing constants.
    /// A section marker drains the running region, so a band never crosses
    /// a divider. Returns the [`Banding`] when the assembled order both
    /// differs from `order` and seats every reference ahead of its
    /// referrer, rewriting `order` in place. Leaves `order` untouched
    /// otherwise.
    pub(super) fn apply(
        self,
        body: &[Stmt],
        sections: &Sections,
        first_party: &[String],
        grouped: bool,
        order: &mut Vec<usize>,
    ) -> Option<Banding> {
        let drain = |region: &mut Vec<usize>, banded: &mut Vec<usize>| {
            self.drain_region(body, first_party, grouped, region, banded);
        };
        let mut banded = Vec::with_capacity(order.len());
        let mut region = Vec::new();
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
        (self.is_sound(&banded) && banded != *order).then(|| {
            *order = banded;
            Banding {
                carries: self.carries,
                ranks: self.ranks,
            }
        })
    }

    /// Appends `region`'s body indices to `out`, the import run sorted to
    /// the front, the leading constants below it, the definitions in
    /// incoming order, the trailing constants last. The import run sorts by
    /// group then name when `grouped`, flat otherwise. Both constant bands
    /// sort by `(tier, name)`. Clears `region`.
    fn drain_region(
        &self,
        body: &[Stmt],
        first_party: &[String],
        grouped: bool,
        region: &mut Vec<usize>,
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
        imports.sort_by_key(|&idx| {
            import_sort_key(&body[idx], first_party, grouped)
                .expect("import band holds only imports")
        });
        leading.sort_by_key(|idx| self.keys[idx]);
        trailing.sort_by_key(|idx| self.keys[idx]);
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
}

/// The band a statement hoists into. `drain_region` seats the bands as
/// imports, leading constants, definitions, then trailing constants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BandRank {
    Definition,
    Import,
    Leading,
    Trailing,
}

/// The gap the banded order seats after the block of rank `a`, ahead of
/// the block of rank `b`. A constant band stays tight, a definition
/// fronts on two blank lines, and an import run keeps one blank line
/// between canonical groups. `None` falls back to the source gap, the
/// case for a pinned anchor on either side, leaving its spacing intact.
pub(super) fn banded_gap(
    ranks: &HashMap<usize, BandRank>,
    body: &[Stmt],
    first_party: &[String],
    grouped: bool,
    a: usize,
    b: usize,
) -> Option<&'static str> {
    Some(match (*ranks.get(&a)?, *ranks.get(&b)?) {
        (BandRank::Leading, BandRank::Leading) | (BandRank::Trailing, BandRank::Trailing) => "\n",
        (BandRank::Import, BandRank::Import)
            if import_blank_lines(&body[a], &body[b], first_party, grouped) == Some(0) =>
        {
            "\n"
        }
        (_, BandRank::Definition) | (BandRank::Definition, _) => "\n\n\n",
        _ => "\n\n",
    })
}

//! Partitions a statement body's slots into sections at each dividing
//! marker, the shared boundary a section-aware reorder consults so it
//! never moves a member across a divider. Import grouping, the family
//! sorts, and constant banding all read one [`Sections`].

use std::ops::Range;

use ruff_text_size::{TextRange, TextSize};

use crate::{
    primitives::comments::{is_banner_block, leading_comment_block},
    source::Source,
};

/// The section partition of a statement body, one slot-index [`Range`]
/// per section. A new section opens at each gap carrying a banner or
/// hash-heading marker, so a body with no marker yields a single
/// section spanning every slot.
pub(crate) struct Sections {
    ranges: Vec<Range<usize>>,
}

impl Sections {
    /// Partitions `blocks` into sections, splitting at each marker-bearing
    /// gap. `blocks` must be in source order.
    pub(crate) fn of(source: &Source, blocks: &[TextRange]) -> Self {
        let mut ranges = Vec::new();
        let mut start = 0;
        for i in 1..blocks.len() {
            if marker_in_gap(source, blocks[i - 1].end(), blocks[i].start()) {
                ranges.push(start..i);
                start = i;
            }
        }
        ranges.push(start..blocks.len());
        Self { ranges }
    }

    /// True when `slot` opens a section past the first, the divider a
    /// same-section reorder never crosses.
    pub(crate) fn is_boundary(&self, slot: usize) -> bool {
        self.ranges.iter().skip(1).any(|range| range.start == slot)
    }

    /// One slot-index range per section, in source order.
    pub(crate) fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}

/// True when a banner or hash heading sits in the gap between two member
/// blocks, opening a section the sort never reorders across.
fn marker_in_gap(source: &Source, lower: TextSize, upper: TextSize) -> bool {
    leading_comment_block(source, lower, upper).is_some_and(|block| is_banner_block(source, block))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::primitives::orderer::member_blocks;
    use crate::testing::parse;

    fn sections_of(source: &Source) -> Sections {
        let body = &source.ast().body;
        let blocks = member_blocks(source, body, source.module_range());
        Sections::of(source, &blocks)
    }

    #[test]
    fn of_splits_at_a_banner_marker() {
        let source = parse("import os\nimport sys\n# --- Typing ---\nimport abc\n");
        let sections = sections_of(&source);
        assert_eq!(sections.ranges(), &[0..2, 2..3]);
    }

    #[test]
    fn of_yields_one_section_without_a_marker() {
        let source = parse("import os\nimport sys\nimport abc\n");
        let sections = sections_of(&source);
        assert_eq!(sections.ranges().len(), 1);
        assert_eq!(sections.ranges()[0], 0..3);
    }

    #[rstest]
    #[case(0, false)]
    #[case(1, true)]
    #[case(2, true)]
    fn is_boundary_marks_only_section_openers(#[case] slot: usize, #[case] expected: bool) {
        let source = parse("x = 1\n# =====\ny = 2\n# =====\nz = 3\n");
        assert_eq!(sections_of(&source).is_boundary(slot), expected);
    }
}

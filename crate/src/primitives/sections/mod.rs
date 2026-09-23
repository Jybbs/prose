//! Partitions a statement body's slots into sections at each comment
//! block left standing between two members and at each notebook cell
//! boundary, the boundaries a section-aware reorder never moves a member
//! across, and around each
//! statement a suppression pins for the rule reordering. Import grouping,
//! the family sorts, and constant banding all read one [`Sections`].

use std::ops::Range;

use ruff_text_size::TextRange;

use crate::{
    primitives::{comments::leading_comment_block, slots::slot_runs},
    rules::RuleId,
    source::Source,
};

/// The section partition of a statement body, one slot-index [`Range`]
/// per section. A new section opens at each gap holding a comment block
/// and at each notebook cell boundary, so a module body with no such
/// block yields a single section spanning every slot.
pub(crate) struct Sections {
    ranges: Vec<Range<usize>>,
}

impl Sections {
    /// Partitions `blocks` into sections, splitting at each gap holding a
    /// comment block and between two members that sit in different
    /// notebook cells.
    /// `blocks` must be in source order.
    pub(crate) fn of(source: &Source, blocks: &[TextRange]) -> Self {
        Self::split(source, blocks, |_| false)
    }

    /// Partitions `blocks` as [`Sections::of`] does, each block a
    /// suppression pins for `rule` sitting in a section of its own, so no
    /// reorder by `rule` moves it or moves a member across it.
    pub(crate) fn pinning(source: &Source, blocks: &[TextRange], rule: RuleId) -> Self {
        let suppression = source.suppression_map();
        if !suppression.has_format_suppression() {
            return Self::of(source, blocks);
        }
        Self::split(source, blocks, |block| suppression.pins(block, rule))
    }

    /// Splits `blocks` at each commented gap, at each notebook cell
    /// boundary, and on both sides of each block `pinned` reports.
    fn split(source: &Source, blocks: &[TextRange], pinned: impl Fn(TextRange) -> bool) -> Self {
        Self {
            ranges: slot_runs(blocks, |&prev, &next| {
                source.same_cell(prev.start(), next.start())
                    && !comment_in_gap(source, TextRange::new(prev.end(), next.start()))
                    && !pinned(prev)
                    && !pinned(next)
            })
            .collect(),
        }
    }

    /// True when `slot` opens a section past the first, the divider a
    /// same-section reorder never crosses.
    pub(crate) fn is_boundary(&self, slot: usize) -> bool {
        slot > 0
            && self
                .ranges
                .binary_search_by_key(&slot, |range| range.start)
                .is_ok()
    }

    /// One slot-index range per section, in source order.
    pub(crate) fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}

/// True when `gap`, the span between two member blocks, holds a comment
/// block neither member binds, one that anchors in place, sits at a
/// shallower indent, or sits behind a notebook cell wall, opening a
/// section the sort never reorders across.
fn comment_in_gap(source: &Source, gap: TextRange) -> bool {
    leading_comment_block(source, gap.start(), gap.end()).is_some()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::primitives::orderer::member_blocks;
    use crate::testing::{notebook, parse};

    fn sections_of(source: &Source) -> Sections {
        let body = &source.ast().body;
        let blocks = member_blocks(source, body, source.module_range());
        Sections::of(source, &blocks)
    }

    #[test]
    fn of_splits_at_a_cell_boundary() {
        let source = notebook(&["import os\nimport sys", "import abc"]);
        let sections = sections_of(&source);
        assert_eq!(sections.ranges(), &[0..2, 2..3]);
    }

    #[rstest]
    #[case::banner("import os\nimport sys\n# --- Typing ---\nimport abc\n", vec![0..2, 2..3])]
    #[case::pragma("import os\nimport sys\n# isort: split\nimport abc\n", vec![0..2, 2..3])]
    #[case::directive("import os\nimport sys\n# fmt: on\nimport abc\n", vec![0..2, 2..3])]
    #[case::plain_comment_binds("import os\nimport sys\n# note\nimport abc\n", vec![0..3])]
    fn of_splits_at_a_standing_comment(#[case] src: &str, #[case] expected: Vec<Range<usize>>) {
        assert_eq!(sections_of(&parse(src)).ranges(), expected);
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

    #[rstest]
    #[case::skipped("import os\nimport sys  # prose: skip\nimport abc\nimport io\n", vec![0..1, 1..2, 2..4])]
    #[case::skipped_for_another_rule(
        "import os\nimport sys  # prose: skip[align-equals]\nimport abc\n",
        vec![0..3],
    )]
    #[case::unsuppressed("import os\nimport sys\n", vec![0..2])]
    fn pinning_seats_a_pinned_block_in_a_section_of_its_own(
        #[case] src: &str,
        #[case] expected: Vec<Range<usize>>,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        let blocks = member_blocks(&source, body, source.module_range());
        let sections = Sections::pinning(&source, &blocks, RuleId::from("group-imports"));
        assert_eq!(sections.ranges(), expected);
    }
}

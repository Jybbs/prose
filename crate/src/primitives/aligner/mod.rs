//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. Each rule wraps the
//! `AlignWalker` in `walker` and drives it through the `emit_*`
//! methods, with the grouping, hold, and member-construction machinery
//! in the sibling modules. This root carries the `Member` row and the
//! per-rule `Settings` knobs both of those share. Aligned rows always
//! carry a one-space buffer between content and the aligned token.

use ruff_text_size::{TextRange, TextSize};

use crate::{
    config::{AlignmentConfig, MaxShift},
    rule::RuleId,
    source::Source,
};

mod emit;
mod grouping;
mod holds;
mod members;
mod walker;

pub(crate) use emit::{operator_columns, space_padding_edit};
pub(crate) use grouping::{
    Slot, adjacent_member_groups, keyed_line_adjacent_groups, line_adjacent_groups,
};
pub(crate) use holds::{is_alignment_candidate, is_held, retain_unheld};
pub(crate) use members::{
    line_anchored_member, line_anchored_member_at_kind, line_anchored_member_between,
    line_gap_before, parameter_split_groups, range_anchored_member_single_line,
};
pub(crate) use walker::AlignWalker;

/// One row in an alignment group.
///
/// `width` is the display-column width of the row's left-hand-side
/// region, from the start of the member to the start of the gap. `gap`
/// is the whitespace range ending immediately before the aligned
/// token that the rule will rewrite. `line_start` is the offset of
/// the start of the source line containing the gap. `op_width` is the
/// display width of the aligned operator itself, used to right-align
/// variable-width operators within a group. Rules with fixed-width
/// operators leave `op_width` at zero. `value_gap` is the span from
/// just past the operator to the value, which an aligned row rewrites
/// to one space, and stays `None` for a rule that leaves the
/// post-operator spacing alone.
#[derive(Clone, Copy)]
pub(crate) struct Member {
    pub gap: TextRange,
    pub line_start: TextSize,
    pub op_width: usize,
    pub value_gap: Option<TextRange>,
    pub width: usize,
}

impl Member {
    /// The post-operator gap an aligned row rewrites to one space,
    /// `None` when the rule leaves that spacing alone or the value
    /// opens on a later line.
    pub(crate) fn rewritten_value_gap(self, source: &Source) -> Option<TextRange> {
        self.value_gap
            .filter(|gap| !source.contains_line_break(*gap))
    }

    /// This member's alignment slot, bridging the run when its row is
    /// skip-held for `rule` so neighbors align around it.
    pub(crate) fn slot(self, source: &Source, rule: RuleId) -> Slot<Self> {
        if is_held(source, rule, self.line_start) {
            Slot::Bridge
        } else {
            Slot::Member(self)
        }
    }

    /// Returns a copy of `self` with `op_width` set to the operator's
    /// display width, opting the member into right-alignment math.
    pub(crate) fn with_op_width(mut self, op_width: usize) -> Self {
        self.op_width = op_width;
        self
    }

    /// Returns a copy of `self` carrying the post-operator span an
    /// aligned row rewrites to one space, running from just past the
    /// `op_len`-wide operator to `value_start`.
    pub(crate) fn with_value_gap(mut self, op_len: TextSize, value_start: TextSize) -> Self {
        self.value_gap = Some(TextRange::new(self.gap.end() + op_len, value_start));
        self
    }
}

/// Emission knobs shared by every alignment rule.
///
/// `max_shift` caps the run's width spread. `strip_singleton`
/// collapses a size-one group's gap to zero width. `line_length`
/// carries the governing cap when the rule resolves within it, so a
/// member whose aligned line would cross the cap partitions out of the
/// run the way an over-`max_shift` outlier does.
#[derive(Clone, Copy)]
pub(crate) struct Settings {
    line_length: Option<usize>,
    max_shift: MaxShift,
    strip_singleton: bool,
}

impl Settings {
    /// Builds the alignment settings carried by an alignment rule, with
    /// `strip_singleton` off and no line cap until a rule opts in.
    fn aligned(max_shift: MaxShift) -> Self {
        Self {
            line_length: None,
            max_shift,
            strip_singleton: false,
        }
    }

    /// Returns the gap width before the aligned token for a group of
    /// `member_count` rows, zero for a stripped singleton and one
    /// space otherwise.
    fn suffix_len(self, member_count: usize) -> usize {
        usize::from(member_count != 1 || !self.strip_singleton)
    }

    /// Returns a copy of `self` carrying `cap` as the governing line
    /// length the run resolves within.
    pub(crate) fn with_line_length(mut self, cap: usize) -> Self {
        self.line_length = Some(cap);
        self
    }

    /// Returns a copy of `self` with `strip_singleton` enabled.
    pub(crate) fn with_singleton_strip(mut self) -> Self {
        self.strip_singleton = true;
        self
    }
}

impl From<&AlignmentConfig> for Settings {
    fn from(c: &AlignmentConfig) -> Self {
        Self::aligned(c.max_shift)
    }
}

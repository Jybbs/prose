//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. Each rule wraps the
//! `AlignWalker` in `walker` and drives it through the `emit_*`
//! methods, with the grouping, hold, and member-construction machinery
//! in the sibling modules. This root carries the `Member` row and the
//! per-rule `Settings` knobs both of those share. An aligned row
//! carries its settings' buffer between content and the aligned token,
//! one space unless the rule widens it.

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
mod widen;

pub(crate) use emit::{operator_columns, space_padding_edit};
pub(crate) use grouping::{
    Slot, adjacent_member_groups, keyed_line_adjacent_groups, line_adjacent_groups,
};
pub(crate) use holds::{is_alignment_candidate, is_held, retain_unheld, shares_column};
pub(crate) use members::{
    line_anchored_member, line_anchored_member_at_kind, line_anchored_member_between,
    line_gap_before, parameter_split_groups, range_anchored_member_single_line,
};
pub(crate) use walker::AlignWalker;
pub(crate) use widen::Widenings;

/// One row in an alignment group.
///
/// `gap` is the whitespace ending immediately before the aligned token
/// the rule rewrites, on the source line opening at `line_start`.
/// `width` measures the row's left-hand side as the source carries it
/// and `settled_width` measures it once an earlier column has padded
/// the content ahead of the gap, so the column math reads the second
/// while the baseline reads the first. `op_width` right-aligns a
/// variable-width operator and stays zero otherwise. `value_gap` runs
/// from just past the operator to the value, `None` where the rule
/// leaves that spacing alone.
#[derive(Clone, Copy)]
pub(crate) struct Member {
    pub gap: TextRange,
    pub line_start: TextSize,
    pub op_width: usize,
    pub settled_width: usize,
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

    /// Returns a copy of `self` measuring its left-hand side at `width`
    /// for the column math, the width the row carries once an earlier
    /// column has padded the content ahead of the gap.
    pub(crate) fn with_settled_width(mut self, width: usize) -> Self {
        self.settled_width = width;
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
/// `buffer` is the gap an aligned row holds between its content and the
/// aligned token. `max_shift` caps the run's width spread.
/// `strip_singleton` collapses a size-one group's gap to zero width.
/// `line_length` carries the governing cap when the rule resolves
/// within it, so a member whose aligned line would cross the cap
/// partitions out of the run the way an over-`max_shift` outlier does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Settings {
    buffer: usize,
    line_length: Option<usize>,
    max_shift: MaxShift,
    strip_singleton: bool,
}

impl Settings {
    /// Builds the alignment settings carried by an alignment rule, with
    /// a one-space buffer, `strip_singleton` off, and no line cap until
    /// a rule opts in.
    fn aligned(max_shift: MaxShift) -> Self {
        Self {
            buffer: 1,
            line_length: None,
            max_shift,
            strip_singleton: false,
        }
    }

    /// Returns the gap width before the aligned token for a group of
    /// `member_count` rows, zero for a stripped singleton and the
    /// settings' buffer otherwise.
    fn suffix_len(self, member_count: usize) -> usize {
        if member_count == 1 && self.strip_singleton {
            0
        } else {
            self.buffer
        }
    }

    /// Returns a copy of `self` carrying `width` as the gap an aligned
    /// row holds ahead of the aligned token.
    pub(crate) fn with_buffer(mut self, width: usize) -> Self {
        self.buffer = width;
        self
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

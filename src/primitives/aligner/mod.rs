//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. Each rule wraps an
//! `AlignWalker` whose `emit_group` method drives the math. Per-rule
//! knobs travel through `Settings`. Aligned rows always carry a
//! one-space buffer between content and the aligned token.

use ruff_diagnostics::Edit;
use ruff_text_size::{TextRange, TextSize};

use crate::{
    config::{AlignmentConfig, MaxShift},
    rule::RuleId,
    source::Source,
};

mod emit;
mod groups;

use emit::emit_group;
pub(crate) use emit::space_padding_edit;
pub(crate) use groups::{
    Slot, adjacent_member_groups, is_alignment_candidate, is_held, keyed_line_adjacent_groups,
    line_adjacent_groups, line_anchored_member, line_anchored_member_at_kind,
    parameter_split_groups, range_anchored_member_single_line,
};

/// Bundles the `groups` accumulator, `settings`, the owning `rule`, and
/// borrowed `source` shared by every alignment-rule visitor. Each entry
/// in `groups` is one fix the pipeline maps to a single diagnostic. The
/// `rule` id powers the skip-directive check that holds a row out of
/// its group.
pub(crate) struct AlignWalker<'a> {
    pub groups: Vec<Vec<Edit>>,
    pub rule: RuleId,
    pub settings: Settings,
    pub source: &'a Source,
}

impl<'a> AlignWalker<'a> {
    /// Builds a walker with an empty `groups` accumulator.
    pub(crate) fn new(source: &'a Source, settings: Settings, rule: RuleId) -> Self {
        Self {
            groups: Vec::new(),
            rule,
            settings,
            source,
        }
    }

    /// Aligns `members` as one fix group, recording it when the pass
    /// rewrites at least one gap.
    pub(crate) fn emit_group(&mut self, members: &[Member]) {
        let edits = self.group_edits(members);
        self.push_group(edits);
    }

    /// Drops the held rows from `members`, then emits the survivors as
    /// one group when they still form an alignment candidate.
    pub(crate) fn emit_unheld(&mut self, members: impl IntoIterator<Item = Member>) {
        let kept: Vec<Member> = members
            .into_iter()
            .filter(|m| !self.is_held(m.line_start))
            .collect();
        if is_alignment_candidate(self.source, &kept) {
            self.emit_group(&kept);
        }
    }

    /// Computes the alignment edits for `members` without recording
    /// them, leaving the caller to fold in further edits before
    /// committing the group through [`Self::push_group`].
    pub(crate) fn group_edits(&self, members: &[Member]) -> Vec<Edit> {
        let mut edits = Vec::new();
        emit_group(self.source, members, self.settings, &mut edits);
        edits
    }

    /// Returns `true` when `anchor`'s source line is skip-suppressed for
    /// this rule, so the row drops out of its alignment group as a
    /// transparent hole that neighbors still align around.
    pub(crate) fn is_held(&self, anchor: TextSize) -> bool {
        is_held(self.source, self.rule, anchor)
    }

    /// Records `edits` as one fix group, dropping an empty group so a
    /// no-op pass emits no diagnostic.
    pub(crate) fn push_group(&mut self, edits: Vec<Edit>) {
        if !edits.is_empty() {
            self.groups.push(edits);
        }
    }
}

/// One row in an alignment group.
///
/// `width` is the display-column width of the row's left-hand-side
/// region, from the start of the member to the start of the gap. `gap`
/// is the whitespace range ending immediately before the aligned
/// token that the rule will rewrite. `line_start` is the offset of
/// the start of the source line containing the gap. `op_width` is the
/// display width of the aligned operator itself, used to right-align
/// variable-width operators within a group. Rules with fixed-width
/// operators leave `op_width` at zero.
#[derive(Clone, Copy)]
pub(crate) struct Member {
    pub gap: TextRange,
    pub line_start: TextSize,
    pub op_width: usize,
    pub width: usize,
}

impl Member {
    /// Returns a copy of `self` with `op_width` set to the operator's
    /// display width, opting the member into right-alignment math.
    pub(crate) fn with_op_width(mut self, op_width: usize) -> Self {
        self.op_width = op_width;
        self
    }
}

/// Emission knobs shared by every alignment rule.
///
/// `max_shift` caps the run's width spread. `strip_singleton`
/// collapses a size-one group's gap to zero width.
#[derive(Clone, Copy)]
pub(crate) struct Settings {
    max_shift: MaxShift,
    strip_singleton: bool,
}

impl Settings {
    /// Builds the alignment settings carried by an alignment rule, with
    /// `strip_singleton` off until a rule opts in.
    fn aligned(max_shift: MaxShift) -> Self {
        Self {
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

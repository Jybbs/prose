//! Computes padding widths and emits alignment edits for rules that
//! align a shared token across a group of lines. Each rule wraps an
//! `AlignWalker` and drives it through the `emit_*` methods. Per-rule
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
mod grouping;
mod holds;
mod members;

use emit::emit_group;
pub(crate) use emit::{operator_columns, space_padding_edit};
pub(crate) use grouping::{
    Slot, adjacent_member_groups, keyed_line_adjacent_groups, line_adjacent_groups,
};
pub(crate) use holds::{is_alignment_candidate, is_held, retain_unheld};
pub(crate) use members::{
    line_anchored_member, line_anchored_member_at_kind, line_anchored_member_between,
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
    fn emit_group(&mut self, members: &[Member]) {
        let edits = self.group_edits(members);
        self.push_group(edits);
    }

    /// Aligns `members` to their shared column and folds in a one-space
    /// rewrite of each gap in `gaps`, recording the combined fix as one
    /// group. The members-level analog of [`Self::push_with_gaps`],
    /// pairing the column math of [`Self::emit_group`] with the gap
    /// normalization.
    pub(crate) fn emit_group_with_gaps(
        &mut self,
        members: &[Member],
        gaps: impl IntoIterator<Item = TextRange>,
    ) {
        let name_edits = self.group_edits(members);
        self.push_with_gaps(name_edits, gaps);
    }

    /// Aligns `members` as one fix group when they form an alignment
    /// candidate, recording nothing otherwise.
    pub(crate) fn emit_if_candidate(&mut self, members: &[Member]) {
        if is_alignment_candidate(self.source, members) {
            self.emit_group(members);
        }
    }

    /// Drops the held rows from `members`, then emits the survivors as
    /// one group when they still form an alignment candidate.
    pub(crate) fn emit_unheld(&mut self, members: impl IntoIterator<Item = Member>) {
        let kept = self.retain_unheld(members, |m| m.line_start);
        self.emit_if_candidate(&kept);
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

    /// Records `name_edits` together with a one-space rewrite of each gap
    /// in `gaps` as one fix group. A gap already holding one space emits
    /// nothing. The gaps are the secondary spans a rule normalizes beside
    /// its aligned column, like the `=`-to-value gap or the `:`-to-body
    /// gap.
    pub(crate) fn push_with_gaps(
        &mut self,
        mut name_edits: Vec<Edit>,
        gaps: impl IntoIterator<Item = TextRange>,
    ) {
        name_edits.extend(
            gaps.into_iter()
                .filter_map(|r| space_padding_edit(self.source, r, 1)),
        );
        self.push_group(name_edits);
    }

    /// Returns the rows of `members` whose anchor line is not skip-held
    /// for this rule. The walker-bound form of the free [`retain_unheld`].
    pub(crate) fn retain_unheld<M>(
        &self,
        members: impl IntoIterator<Item = M>,
        line_start: impl Fn(&M) -> TextSize,
    ) -> Vec<M> {
        retain_unheld(self.source, self.rule, members, line_start)
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

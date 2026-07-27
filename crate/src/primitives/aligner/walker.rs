//! The emitter facade the alignment rules drive. Each rule wraps an
//! `AlignWalker` and calls the `emit_*` methods, which pair the column
//! math with the skip-hold check and the gap normalization.

use ruff_diagnostics::Edit;
use ruff_text_size::{TextRange, TextSize};

use super::{
    Member, Settings, emit::emit_group, is_alignment_candidate, is_held, retain_unheld,
    space_padding_edit,
};
use crate::{rule::RuleId, source::Source};

/// Bundles the `groups` accumulator, `settings`, the owning `rule`, and
/// borrowed `source` shared by every alignment-rule visitor. Each entry
/// in `groups` is one fix the pipeline maps to a single diagnostic. The
/// `rule` id powers the skip-directive check that holds a row out of
/// its group.
pub(crate) struct AlignWalker<'a> {
    pub groups: Vec<Vec<Edit>>,
    pub rule: RuleId,
    settings: Settings,
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

    /// Computes the alignment edits for `members` under `settings`
    /// rather than the walker's own.
    fn group_edits_under(&self, settings: Settings, members: &[Member]) -> Vec<Edit> {
        let mut edits = Vec::new();
        emit_group(self.source, members, settings, &mut edits);
        edits
    }

    /// Aligns `members` to their shared column and folds in a one-space
    /// rewrite of each gap in `gaps`, recording the combined fix as one
    /// group. The members-level analog of [`Self::push_with_gaps`],
    /// pairing the column math of [`Self::group_edits`] with the gap
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
    /// candidate, folding in a one-space rewrite of each member's
    /// [post-operator gap](Self::value_gaps). Records nothing otherwise.
    pub(crate) fn emit_if_candidate(&mut self, members: &[Member]) {
        self.emit_if_candidate_under(self.settings, members);
    }

    /// Aligns `members` as one fix group under `settings` rather than the
    /// walker's own, folding in the same post-operator gap rewrite
    /// [`Self::emit_if_candidate`] applies. Records nothing when
    /// `members` form no alignment candidate.
    pub(crate) fn emit_if_candidate_under(&mut self, settings: Settings, members: &[Member]) {
        if is_alignment_candidate(self.source, members) {
            let gaps = self.value_gaps(members);
            let name_edits = self.group_edits_under(settings, members);
            self.push_with_gaps(name_edits, gaps);
        }
    }

    /// Drops the held rows from `members`, then emits the survivors as
    /// one group when they still form an alignment candidate.
    pub(crate) fn emit_unheld(&mut self, members: impl IntoIterator<Item = Member>) {
        let kept = retain_unheld(self.source, self.rule, members);
        self.emit_if_candidate(&kept);
    }

    /// Computes the alignment edits for `members` without recording
    /// them, leaving the caller to fold in further edits before
    /// committing the group through [`Self::push_group`].
    pub(crate) fn group_edits(&self, members: &[Member]) -> Vec<Edit> {
        self.group_edits_under(self.settings, members)
    }

    /// Returns `true` when `anchor`'s source line is skip-suppressed for
    /// this rule.
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

    /// The post-operator gaps this rule rewrites to one space, one per
    /// member whose value shares the operator's line.
    pub(crate) fn value_gaps(&self, members: &[Member]) -> Vec<TextRange> {
        members
            .iter()
            .filter_map(|m| m.rewritten_value_gap(self.source))
            .collect()
    }
}

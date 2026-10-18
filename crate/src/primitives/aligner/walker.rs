//! The emitter facade the alignment rules drive. Each rule wraps an
//! `AlignWalker` and calls the `emit_*` methods, which pair the column
//! math with the skip-hold check and the gap normalization.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    Member, Settings, Widenings,
    emit::{emitted_extents, group_paddings},
    holds::shares_column,
    is_held, operator_columns, retain_unheld, space_padding_edit,
};
use crate::{
    primitives::{
        edit::{apply_inline_edits, insert_edit},
        inline::display_width,
        reseat::RowMoves,
    },
    rules::RuleId,
    source::Source,
};

/// Bundles the `groups` accumulator, `settings`, the owning `rule`, and
/// borrowed `source` shared by every alignment-rule visitor. Each entry
/// in `groups` is one fix the pipeline maps to a single diagnostic. The
/// skip-directive check reads `rule` to hold a row out of its group.
pub(crate) struct AlignWalker<'a> {
    groups: Vec<Vec<Edit>>,
    /// The continuation rows the padded values carry, each filed with the
    /// group that first moves it once the walk ends.
    moves: RowMoves,
    /// Every edit in `groups` again, ordered by start for the prefix
    /// lookup's binary search.
    placed: Vec<Edit>,
    pub rule: RuleId,
    settings: Settings,
    pub source: &'a Source,
    widenings: Widenings,
}

impl<'a> AlignWalker<'a> {
    /// Builds a walker with an empty `groups` accumulator.
    pub(crate) fn new(source: &'a Source, settings: Settings, rule: RuleId) -> Self {
        Self {
            groups: Vec::new(),
            moves: RowMoves::default(),
            placed: Vec::new(),
            rule,
            settings,
            source,
            widenings: Widenings::default(),
        }
    }

    /// Computes the alignment edits for `members` under `settings`
    /// rather than the walker's own, per [`Self::padded_edits`].
    fn group_edits_under(&mut self, settings: Settings, members: &[Member]) -> Vec<Edit> {
        let extents = emitted_extents(self.source, members, settings, &self.widenings, &[]);
        self.padded_edits(group_paddings(members, &extents, settings))
    }

    /// True when `members` form a multi-row group whose aligned tokens
    /// share a display column once the edits this walker has already
    /// recorded land.
    fn is_placed_candidate(&self, members: &[Member]) -> bool {
        shares_column(members, |m| self.placed_baseline(m))
    }

    /// Returns the edits padding each member of `paddings` to its paired
    /// width, recording the move of each continuation row a padded value
    /// carries, per [`Member::rows_moved_by`], for the group pushed next.
    /// Each row move widens its row for every later line-cap check.
    fn padded_edits(&mut self, paddings: impl IntoIterator<Item = (Member, usize)>) -> Vec<Edit> {
        let group = self.groups.len();
        paddings
            .into_iter()
            .filter_map(|(member, pad)| {
                let shift = member.value_shift(self.source, pad);
                let rows = member.rows_moved_by(self.source, shift);
                for row in &rows {
                    self.widenings.seat_row(row.start(), shift);
                }
                self.moves.carry(group, &rows, shift);
                space_padding_edit(self.source, member.gap, pad)
            })
            .collect()
    }

    /// Returns the display column where `member`'s left-hand side begins
    /// once the edits this walker has already recorded land on its line
    /// ahead of the gap, the source baseline moved by however much those
    /// edits and the moves of the row widen the prefix.
    /// `apply_inline_edits` bounds `placed` to that prefix itself, so a
    /// row no recorded edit touches or moves reads its source baseline.
    fn placed_baseline(&self, member: Member) -> usize {
        let prefix = TextRange::new(member.line_start, member.gap.start());
        let placed = apply_inline_edits(self.source, prefix, &self.placed);
        member.baseline.saturating_add_signed(
            display_width(&placed).cast_signed()
                - display_width(self.source.slice(prefix)).cast_signed()
                + self.moves.shift(member.line_start),
        )
    }

    /// Records `name_edits` together with a one-space rewrite of each gap
    /// in `gaps` as one fix group. A gap already holding one space emits
    /// nothing. The gaps are the secondary spans a rule normalizes beside
    /// its aligned column, like the `=`-to-value gap or the `:`-to-body
    /// gap.
    fn push_with_gaps(
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
    fn value_gaps(&self, members: &[Member]) -> Vec<TextRange> {
        members
            .iter()
            .filter_map(|m| m.rewritten_value_gap(self.source))
            .collect()
    }

    /// Returns the alignment edits `members` take under `settings` rather
    /// than the walker's own, empty when they form no alignment candidate.
    /// The caller folds them into the fix group it pushes next rather
    /// than recording them on their own.
    pub(crate) fn candidate_edits_under(
        &mut self,
        settings: Settings,
        members: &[Member],
    ) -> Vec<Edit> {
        if self.is_placed_candidate(members) {
            self.group_edits_under(settings, members)
        } else {
            Vec::new()
        }
    }

    /// Returns the alignment edits `members` take under `settings` for the
    /// fix group pushed next, their shared column where they form an
    /// alignment candidate and each member's own buffer otherwise.
    pub(crate) fn column_or_buffer_edits(
        &mut self,
        settings: Settings,
        members: &[Member],
    ) -> Vec<Edit> {
        if self.is_placed_candidate(members) {
            return self.group_edits_under(settings, members);
        }
        self.padded_edits(members.iter().map(|&m| (m, settings.buffer())))
    }

    /// Records [`Self::column_or_buffer_edits`] for `members` as one
    /// fix group, folding in the post-operator gaps.
    pub(crate) fn emit_group_or_buffer(&mut self, members: &[Member]) {
        let name_edits = self.column_or_buffer_edits(self.settings, members);
        let gaps = self.value_gaps(members);
        self.push_with_gaps(name_edits, gaps);
    }

    /// Aligns `members` to their shared column and folds in a one-space
    /// rewrite of each gap in `gaps`, recording the combined fix as one
    /// group. The members-level analog of [`Self::push_with_gaps`],
    /// pairing the column math with the gap normalization.
    pub(crate) fn emit_group_with_gaps(
        &mut self,
        members: &[Member],
        gaps: impl IntoIterator<Item = TextRange>,
    ) {
        let name_edits = self.group_edits_under(self.settings, members);
        self.push_with_gaps(name_edits, gaps);
    }

    /// Aligns `members` as one fix group when they form an alignment
    /// candidate, folding in a one-space rewrite of each member's
    /// [post-operator gap](Self::value_gaps). Records nothing otherwise.
    pub(crate) fn emit_if_candidate(&mut self, members: &[Member]) {
        if self.is_placed_candidate(members) {
            let gaps = self.value_gaps(members);
            self.emit_group_with_gaps(members, gaps);
        }
    }

    /// Drops the held rows from `members`, then emits the survivors as
    /// one group when they still form an alignment candidate.
    pub(crate) fn emit_unheld(&mut self, members: impl IntoIterator<Item = Member>) {
        let kept = retain_unheld(self.source, self.rule, members);
        self.emit_if_candidate(&kept);
    }

    /// Returns the fix groups the walk recorded, each carrying the moves
    /// of the continuation rows it first moves, where one carried value
    /// nesting inside another moves a row both carry once by their
    /// summed shift. An empty group is dropped, so a no-op pass emits no
    /// diagnostic.
    pub(crate) fn into_groups(self) -> Vec<Vec<Edit>> {
        let mut groups = self.groups;
        self.moves.file_into(self.source, &mut groups);
        groups.retain(|group| !group.is_empty());
        groups
    }

    /// Returns `true` when `anchor`'s source line is skip-suppressed for
    /// this rule.
    pub(crate) fn is_held(&self, anchor: TextSize) -> bool {
        is_held(self.source, self.rule, anchor)
    }

    /// Returns the display column each member's aligned token lands on
    /// under this walker's settings and widenings, per the free
    /// [`operator_columns`].
    pub(crate) fn operator_columns(&self, members: &[Member]) -> Vec<usize> {
        operator_columns(self.source, members, self.settings, &self.widenings, &[])
    }

    /// Records `edits` as one fix group, which also takes every row move
    /// recorded while the edits were computed.
    pub(crate) fn push_group(&mut self, edits: Vec<Edit>) {
        for edit in &edits {
            insert_edit(&mut self.placed, edit.clone());
        }
        self.groups.push(edits);
    }

    /// Installs the widening entries the rule's collected groups seat,
    /// read by every later line-cap check.
    pub(crate) fn set_widenings(&mut self, widenings: Widenings) {
        self.widenings = widenings;
    }
}

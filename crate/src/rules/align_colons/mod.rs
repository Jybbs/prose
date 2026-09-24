//! Pads the space before `:` so consecutive dict entries, annotated
//! assignments, annotated parameters, and docstring entries share one
//! column, leaving a lone row, a one-line group, or rows at different
//! baselines to `strip_stranded_padding`. Every aligned `:` keeps one
//! space before it. In the dict, annotation, and parameter contexts the
//! space after it shrinks to one, and a row that would run past
//! `code_line_length` starts a new column. A docstring run aligns under
//! no line limit, padding its parenthesized types onto a column of their
//! own before the `:` column is measured, and resolves as it would under
//! `max-shift = 0` when `align-docstring-entries` is off.

use ruff_diagnostics::Edit;

use crate::{
    config::{Config, MaxShift},
    primitives::{
        aligner,
        colon_targets::{ColonEmitter, EntryColumns},
        reserve,
    },
    rules::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct AlignColons {
    docstring_settings: aligner::Settings,
    reservations: reserve::Reservations,
    settings: aligner::Settings,
    type_settings: aligner::Settings,
}

impl AlignColons {
    pub(crate) const MESSAGE: &'static str = "align consecutive `:` separators";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) const PRESERVES_TREE: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let align_colons = &config.rules.align_colons;
        let docstring_shift = if align_colons.align_docstring_entries {
            align_colons.max_shift
        } else {
            MaxShift::NoShift
        };
        let type_settings = aligner::Settings::aligned(docstring_shift);
        Self {
            docstring_settings: type_settings.with_singleton_strip(),
            reservations: config.equals_reservations(),
            settings: config
                .align_settings(align_colons.max_shift, config.code_width())
                .with_singleton_strip(),
            type_settings,
        }
    }
}

impl Rule for AlignColons {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut emitter = Emitter {
            rule: self,
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        emitter
            .walker
            .set_widenings(self.reservations.widenings(source));
        emitter.walk(source);
        emitter.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Emitter<'a> {
    rule: &'a AlignColons,
    walker: aligner::AlignWalker<'a>,
}

impl ColonEmitter for Emitter<'_> {
    /// Seats the type-group column first, then the `:` column against
    /// the widths that padding leaves, recording both as one fix group.
    /// The type-group column takes no singleton strip, collapsing to
    /// its one-space buffer.
    fn docstring_entries(&mut self, run: &EntryColumns) {
        let (parens, colons) = run.settled_columns(self.walker.source, self.rule.type_settings);
        let mut edits = self
            .walker
            .column_or_buffer_edits(self.rule.type_settings, &parens);
        edits.extend(
            self.walker
                .candidate_edits_under(self.rule.docstring_settings, &colons),
        );
        self.walker.push_group(edits);
    }

    fn handle(&mut self, members: &[aligner::Member]) {
        self.walker.emit_if_candidate(members);
    }

    fn match_arms(&mut self, _: &[aligner::Member]) {}

    fn rule(&self) -> RuleId {
        self.walker.rule
    }
}

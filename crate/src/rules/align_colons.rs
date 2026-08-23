//! Aligns `:` vertically in dict/mapping literals, annotated
//! assignments, annotated function parameters, and docstring entry
//! runs. Single-line groups, single-item groups, and groups whose rows
//! open at differing baselines pass through to `strip_stranded_padding`.
//! Each aligned `:` keeps a one-space buffer before it, and the dict,
//! annotation, and parameter contexts collapse the gap after it to one
//! space and resolve within `code_line_length`, whereas a docstring run
//! carries no cap. A docstring run settles its parenthesized type
//! groups into a column first, so the `:` column measures the widths
//! that padding leaves, and a `(` flush against its name documents a
//! call rather than a type, opening no type column while its `:` still
//! settles with the run.

use ruff_diagnostics::Edit;

use crate::{
    config::Config,
    primitives::{
        aligner,
        colon_targets::{ColonEmitter, EntryColumns},
        reserve,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignColons {
    docstring_settings: aligner::Settings,
    reservations: reserve::Reservations,
    settings: aligner::Settings,
    type_settings: aligner::Settings,
}

impl AlignColons {
    pub(crate) const MESSAGE: &'static str = "align consecutive `:` separators";

    pub(crate) fn from_config(config: &Config) -> Self {
        let type_settings = aligner::Settings::from(&config.rules.align_colons);
        let docstring_settings = type_settings.with_singleton_strip();
        Self {
            docstring_settings,
            reservations: config.equals_reservations(),
            settings: docstring_settings.within(
                config.code_width(),
                config.stranded_padding(),
                config.comment_settling(),
            ),
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
    /// the widths that padding leaves, recording both as one fix group
    /// so a run settles in a single pass. The type-group column takes
    /// no singleton strip, collapsing instead to its one-space buffer.
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

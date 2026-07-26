//! Aligns `:` vertically in dict/mapping literals, annotated
//! assignments, annotated function parameters, and Google-style
//! docstring sections. Single-line groups, single-item groups,
//! and groups whose rows open at differing column baselines pass
//! through, leaving them to `strip_align_padding` downstream. Each
//! aligned `:` keeps a one-space buffer before the colon, and the dict,
//! annotation, and parameter contexts collapse the gap after it to one
//! space. The code contexts resolve within `code_line_length`, whereas
//! a docstring section's run carries no cap and reads the column it
//! aligns to once wrapped, since `docstring_wrap` reflows each entry to
//! `docstring_line_length` from the padded column downstream.

use ruff_diagnostics::Edit;

use crate::{
    config::Config,
    primitives::{
        aligner,
        colon_targets::{ColonEmitter, ColonMember},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignColons {
    docstring_settings: aligner::Settings,
    settings: aligner::Settings,
}

impl AlignColons {
    pub(crate) fn from_config(config: &Config) -> Self {
        let docstring_settings =
            aligner::Settings::from(&config.rules.align_colons).with_singleton_strip();
        Self {
            docstring_settings,
            settings: docstring_settings.with_line_length(config.code_width()),
        }
    }
}

impl Rule for AlignColons {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut emitter = Emitter {
            docstring_settings: self.docstring_settings,
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        emitter.walk(source);
        emitter.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Emitter<'a> {
    docstring_settings: aligner::Settings,
    walker: aligner::AlignWalker<'a>,
}

impl Emitter<'_> {
    /// Aligns `members` under `settings`, rewriting each single-line
    /// post-colon gap to one space in the same fix group.
    fn emit(&mut self, members: &[ColonMember], settings: aligner::Settings) {
        let source = self.walker.source;
        let aligned: Vec<aligner::Member> = members.iter().map(|m| m.member).collect();
        let value_gaps = members
            .iter()
            .filter_map(|m| m.single_line_value_gap(source));
        self.walker
            .emit_if_candidate_under(settings, &aligned, value_gaps);
    }
}

impl ColonEmitter for Emitter<'_> {
    fn docstring_entries(&mut self, members: &[ColonMember]) {
        self.emit(members, self.docstring_settings);
    }

    fn handle(&mut self, members: &[ColonMember]) {
        self.emit(members, self.walker.settings);
    }

    fn match_arms(&mut self, _: &[ColonMember]) {}

    fn rule(&self) -> RuleId {
        self.walker.rule
    }
}

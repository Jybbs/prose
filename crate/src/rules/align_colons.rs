//! Aligns `:` vertically in dict/mapping literals, annotated
//! assignments, annotated function parameters, and Google-style
//! docstring sections. Single-line groups, single-item groups,
//! and groups whose rows open at differing column baselines pass
//! through, leaving them to `strip_align_padding` downstream. Each
//! aligned `:` keeps a one-space buffer before the colon, and the dict,
//! annotation, and parameter contexts collapse the gap after it to one
//! space.

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

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
    settings: aligner::Settings,
}

impl AlignColons {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_colons).with_singleton_strip(),
        }
    }
}

impl Rule for AlignColons {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut emitter = Emitter {
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
    walker: aligner::AlignWalker<'a>,
}

impl ColonEmitter for Emitter<'_> {
    fn handle(&mut self, members: &[ColonMember]) {
        let source = self.walker.source;
        let aligned: Vec<aligner::Member> = members.iter().map(|m| m.member).collect();
        let value_gaps: Vec<TextRange> = members
            .iter()
            .filter_map(|m| m.value_gap)
            .filter(|gap| !source.contains_line_break(*gap))
            .collect();
        self.walker
            .emit_if_candidate_with_gaps(&aligned, value_gaps);
    }

    fn match_arms(&mut self, _: &[ColonMember]) {}

    fn rule(&self) -> RuleId {
        self.walker.rule
    }
}

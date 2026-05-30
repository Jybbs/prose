//! Aligns `:` vertically in dict/mapping literals, Pydantic-style
//! class fields, annotated function parameters, and Google/numpy
//! docstring `Args:` sections. Single-line groups and single-item
//! groups pass through, leaving the latter to `singleton_rule`
//! downstream. Each aligned `:` keeps a one-space buffer before the
//! colon.

use ruff_diagnostics::Edit;
use ruff_python_ast::ExprDict;
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    primitives::{aligner, colon_targets::ColonEmitter},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignColons {
    settings: aligner::Settings,
}

impl AlignColons {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_colons)
                .with_singleton_subgroup_strip(),
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
    fn dict(&mut self, d: &ExprDict, members: &[aligner::Member]) {
        let source = self.walker.source;
        let rule = self.walker.rule;
        let dict_range = d.range();
        let has_real_comment = source
            .comment_ranges()
            .iter()
            .any(|c| dict_range.contains_range(*c) && !aligner::is_held(source, rule, c.start()));
        if !has_real_comment {
            self.handle(members);
        }
    }

    fn handle(&mut self, members: &[aligner::Member]) {
        if aligner::is_alignment_candidate(members) {
            self.walker.emit_group(members);
        }
    }

    fn match_arms(&mut self, _: &[aligner::Member]) {}

    fn rule(&self) -> RuleId {
        self.walker.rule
    }
}

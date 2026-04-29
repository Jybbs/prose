//! Aligns `:` vertically in dict/mapping literals, Pydantic-style
//! class fields, annotated function parameters, and Google/numpy
//! docstring `Args:` sections. Single-line groups and single-item
//! groups pass through, leaving the latter to `singleton_rule`
//! downstream. Each aligned `:` keeps a one-space buffer before the
//! colon.

use ruff_diagnostics::Edit;
use ruff_python_ast::ExprDict;

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::aligner;
use crate::primitives::colon_targets::ColonEmitter;
use crate::source::Source;

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
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut emitter = Emitter {
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        emitter.walk(source);
        emitter.edits
    }

    fn name(&self) -> &'static str {
        "align-colons"
    }
}

struct Emitter<'a> {
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl ColonEmitter for Emitter<'_> {
    fn handle(&mut self, members: &[aligner::Member]) {
        if aligner::is_alignment_candidate(members) {
            aligner::emit_group(self.source, members, self.settings, &mut self.edits);
        }
    }

    fn dict(&mut self, d: &ExprDict, members: &[aligner::Member]) {
        if !self.source.intersects_comment(d) {
            self.handle(members);
        }
    }

    fn match_arms(&mut self, _: &[aligner::Member]) {}
}

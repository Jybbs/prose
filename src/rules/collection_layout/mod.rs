//! Lays out `dict`, `list`, `set`, and `tuple` literals against the
//! `Config::code_line_length` budget. Multi-line literals whose
//! assembled inline form fits collapse back to a single line, as does
//! a multi-line subscript whose `value[index]` fits and a multi-line
//! collection or subscript dict key, so the alignment rules meet a
//! single-line member rather than one stranded across a break.
//! Single-line literals whose inline form overflows expand to one
//! entry per line. A dict holding more entries than
//! `max_inline_dict_entries` expands whatever its width, taking any
//! enclosing collection with it. A dict entry whose `key: value`
//! width overflows at the item-indent column breaks at `:` and hangs
//! the value at `item_indent + INDENT_STEP`. Comprehensions and any
//! literal whose source range contains a comment are out of scope.
//!
//! Both fit checks stay invariant to the alignment that runs later: a
//! dict entry measures at its canonical `": "` rather than an
//! `align_colons`-padded gap, and a collapse tests against the column
//! `align_equals` shifts the value's `=` to.

use std::{collections::HashMap, num::NonZeroUsize};

use ruff_diagnostics::Edit;
use ruff_python_ast::{helpers::any_over_body, visitor::Visitor};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    primitives::{aligner, edit::singleton_groups},
    rule::{Rule, RuleId},
    source::Source,
};

mod classify;
mod flow;
mod layouter;
mod reserve;

use layouter::Layouter;

pub(crate) struct CollectionLayout {
    align_equals: Option<aligner::Settings>,
    code_line_length: usize,
    max_atomics_per_line: usize,
    max_inline_dict_entries: Option<usize>,
}

impl CollectionLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.collection_layout;
        let align_equals = &config.rules.align_equals;
        Self {
            // Reserve the column `align_equals` shifts a value to only when
            // it runs, since a disabled rule leaves the `=` unaligned.
            align_equals: align_equals
                .enabled
                .then(|| aligner::Settings::from(align_equals)),
            code_line_length: config.code_width(),
            max_atomics_per_line: rules
                .max_atomics_per_line
                .map_or(usize::MAX, NonZeroUsize::get),
            max_inline_dict_entries: rules.max_inline_dict_entries.map(NonZeroUsize::get),
        }
    }
}

impl Rule for CollectionLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        // Precomputed once so the per-node count check is a containment
        // scan rather than re-walking each subtree.
        let tripping_dicts = self.max_inline_dict_entries.map_or_else(Vec::new, |cap| {
            let mut ranges = Vec::new();
            any_over_body(body, |expr| {
                if expr.as_dict_expr().is_some_and(|dict| dict.len() > cap) {
                    ranges.push(expr.range());
                }
                false
            });
            ranges
        });
        let reservations = self.align_equals.map_or_else(HashMap::new, |settings| {
            reserve::reserved_columns(source, settings)
        });
        let mut visitor = Layouter {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_atomics_per_line: self.max_atomics_per_line,
            newline: source.newline_str(),
            reservations,
            source,
            tripping_dicts,
        };
        visitor.visit_body(body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

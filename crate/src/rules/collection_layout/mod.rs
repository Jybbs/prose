//! Lays out `dict`, `list`, `set`, and `tuple` literals against the
//! `Config::code_line_length` budget. A multi-line subscript,
//! comprehension, or dict key whose inline form fits rejoins onto one
//! line whatever the facets hold. An overflowing single-line literal
//! expands one entry per line, and a dict over `max_dict_entries`
//! expands whatever its width, taking any enclosing collection with it.
//! An over-wide dict entry breaks at `:` and hangs its value. A
//! subscript and a comprehension only ever rejoin, and a comment, an
//! f-string or t-string replacement field, or a folded multi-line
//! string holds a construct at its source shape.
//!
//! `keep_multiline_literals` holds a literal the author laid out as a
//! flush bracketed column of two or more entries, so it re-expands to
//! the canonical shape rather than joining, and a held literal keeps
//! its break inside any enclosing rejoin. Every other break is a
//! fracture and rejoins either way.
//!
//! Both fit checks stay invariant to the later alignment: a dict entry
//! measures at its canonical `": "`, and a rejoin tests against the
//! column `align_equals` shifts the value to.

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    primitives::{
        aligner, edit::singleton_groups, reserve::reserved_columns, walk::filter_map_over_exprs,
    },
    rule::{Rule, RuleId},
    rules::align_equals::AlignEquals,
    source::Source,
};

mod classify;
mod flow;
mod inline;
mod layouter;

use layouter::Layouter;

pub(crate) struct CollectionLayout {
    align_equals: Option<aligner::Settings>,
    code_line_length: usize,
    explode: bool,
    keep_multiline_literals: bool,
    max_atomics: usize,
    max_dict_entries: Option<usize>,
    wrap_dict_entries: bool,
}

impl CollectionLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.collection_layout;
        Self {
            align_equals: AlignEquals::reserve_settings(config),
            code_line_length: config.code_width(),
            explode: rules.explode,
            keep_multiline_literals: rules.keep_multiline_literals,
            max_atomics: rules.max_atomics.cap().unwrap_or(usize::MAX),
            max_dict_entries: rules.max_dict_entries.cap(),
            wrap_dict_entries: rules.wrap_dict_entries,
        }
    }
}

impl Rule for CollectionLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        // The count cap reads the `explode` facet, so a cleared `explode`
        // leaves no tripping dicts and the cap goes inert. Precomputed once
        // so the per-node check is a containment scan rather than a re-walk.
        let count_cap = self.max_dict_entries.filter(|_| self.explode);
        let tripping_dicts = count_cap.map_or_else(Vec::new, |cap| {
            filter_map_over_exprs(body, |expr| {
                expr.as_dict_expr()
                    .filter(|dict| dict.len() > cap)
                    .map(Ranged::range)
            })
        });
        let reservations = reserved_columns(source, self.align_equals, AlignEquals::SLUG);
        let mut visitor = Layouter {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            explode: self.explode,
            keep_multiline_literals: self.keep_multiline_literals,
            max_atomics: self.max_atomics,
            newline: source.newline_str(),
            reservations,
            source,
            tripping_dicts,
            wrap_dict_entries: self.wrap_dict_entries,
        };
        visitor.visit_body(body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

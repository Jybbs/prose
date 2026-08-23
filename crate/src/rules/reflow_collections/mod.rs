//! Lays out `dict`, `list`, `set`, and `tuple` literals against the
//! `Config::code_line_length` budget. A multi-line subscript,
//! comprehension, or dict key whose inline form fits rejoins onto one
//! line whatever the facets hold. An overflowing single-line literal
//! expands one entry per line, and a dict over `max_dict_entries`
//! expands whatever its width, taking any enclosing collection with it.
//! An over-wide dict entry breaks at `:` and hangs its value, leaving an
//! entry either side of whose `:` carries an implicitly concatenated
//! string to `stack-adjacent-strings`. A subscript and a comprehension
//! only ever rejoin, and a comment, an f-string or t-string replacement
//! field, or a folded multi-line string holds a construct at its source
//! shape.
//!
//! A member the expansion holds rather than lays out itself moves with
//! the row it lands on, its continuation lines hanging from the item
//! column, and a member running through a multi-line string stays at
//! its source column instead.
//!
//! `keep_multiline_literals` holds a literal the author laid out as a
//! flush bracketed column of two or more entries, so it re-expands to
//! the canonical shape rather than joining, and a held literal keeps
//! its break inside any enclosing rejoin. Every other break is a
//! fracture and rejoins either way.
//!
//! Both fit checks stay invariant to the later alignment: a dict entry
//! measures at its canonical `": "`, and a rejoin tests against the
//! column `align_equals` shifts the value to. The expanded layout stays
//! invariant to the later ordering the same way, every row charged the
//! separator closing it whatever position `alphabetize-siblings` moves
//! it to.

use ruff_diagnostics::Edit;
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    primitives::{
        call_keywords::module_call_params,
        edit::singleton_groups,
        one_row,
        padding::Stranding,
        reserve,
        walk::{filter_map_over_exprs, walk_parented_exprs},
    },
    rule::{Rule, RuleId},
    rules::alphabetize_siblings::Reorders,
    source::Source,
};

mod classify;
mod flow;
mod layouter;

use layouter::Layouter;

pub(crate) struct ReflowCollections {
    code_line_length: usize,
    explode: bool,
    max_atomics: usize,
    one_row: one_row::Settings<'static>,
    reorders: Reorders,
    reservations: reserve::Reservations,
    stranding: Stranding,
    wrap_dict_entries: bool,
}

impl ReflowCollections {
    pub(crate) const MESSAGE: &'static str = "lay out collection literal against the line budget";

    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.reflow_collections;
        Self {
            code_line_length: config.code_width(),
            explode: rules.explode,
            max_atomics: rules.max_atomics.cap().unwrap_or(usize::MAX),
            one_row: config.one_row_settings(),
            reorders: config.reorders(),
            reservations: config.equals_reservations(),
            stranding: config.stranded_padding(),
            wrap_dict_entries: rules.wrap_dict_entries,
        }
    }
}

impl Rule for ReflowCollections {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        // The count cap reads the `explode` facet, so a cleared `explode`
        // leaves no tripping dicts and the cap goes inert. Precomputed once
        // so the per-node check is a containment scan rather than a re-walk.
        let count_cap = self.one_row.dict_entry_cap();
        let tripping_dicts = count_cap.map_or_else(Vec::new, |cap| {
            filter_map_over_exprs(body, |expr| {
                expr.as_dict_expr()
                    .filter(|dict| dict.len() > cap)
                    .map(Ranged::range)
            })
        });
        let targets = module_call_params(source);
        let reservations = source.columns(self.reservations);
        let padding = source.stranded_padding(self.stranding);
        let mut layouter = Layouter {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            explode: self.explode,
            max_atomics: self.max_atomics,
            newline: source.newline_str(),
            one_row: self.one_row.against(&targets),
            padding: &padding,
            reorders: self.reorders,
            reservations: &reservations,
            source,
            targets: &targets,
            tripping_dicts,
            wrap_dict_entries: self.wrap_dict_entries,
        };
        walk_parented_exprs(source.ast(), &mut layouter);
        singleton_groups(layouter.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

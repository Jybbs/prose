//! Lays out `dict`, `list`, `set`, and `tuple` literals against the
//! `Config::code_line_length` budget. A multi-line literal, subscript,
//! or comprehension whose inline form fits collapses to one line. An
//! overflowing single-line literal expands one entry per line, and a
//! dict over `max_dict_entries` expands whatever its width, taking any
//! enclosing collection with it. An over-wide dict entry breaks at `:`
//! and hangs its value. A subscript and a comprehension only ever
//! collapse, and a comment, an f-string or t-string replacement field,
//! or a folded multi-line string holds a construct at its source shape.
//!
//! Both fit checks stay invariant to the later alignment: a dict entry
//! measures at its canonical `": "`, and a collapse tests against the
//! column `align_equals` shifts the value's `=` to.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};

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
    collapse: bool,
    explode: bool,
    max_atomics: usize,
    max_dict_entries: Option<usize>,
    wrap_dict_entries: bool,
}

impl CollectionLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.collection_layout;
        let align_equals = &config.rules.align_equals;
        Self {
            // Reserve the column `align_equals` shifts a value to only when
            // it runs, since a disabled rule leaves the `=` unaligned.
            align_equals: align_equals.enabled.then(|| {
                aligner::Settings::from(align_equals).with_line_length(config.code_width())
            }),
            code_line_length: config.code_width(),
            collapse: rules.collapse,
            explode: rules.explode,
            max_atomics: rules.max_atomics.cap().unwrap_or(usize::MAX),
            max_dict_entries: rules.max_dict_entries.cap(),
            wrap_dict_entries: rules.wrap_dict_entries,
        }
    }
}

impl Rule for CollectionLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        // The count cap applies only while `explode` is set, so a cleared
        // `explode` leaves no tripping dicts and the cap goes inert. It is
        // precomputed once, leaving the per-node check a containment scan.
        let count_cap = self.max_dict_entries.filter(|_| self.explode);
        let tripping_dicts = count_cap.map_or_else(Vec::new, |cap| over_count_dicts(body, cap));
        let reservations = self.align_equals.map_or_else(HashMap::new, |settings| {
            reserve::reserved_columns(source, settings)
        });
        let mut visitor = Layouter {
            code_line_length: self.code_line_length,
            collapse: self.collapse,
            edits: Vec::new(),
            explode: self.explode,
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

struct DictScan {
    cap: usize,
    ranges: Vec<TextRange>,
}

impl<'a> Visitor<'a> for DictScan {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(dict) = expr.as_dict_expr()
            && dict.len() > self.cap
        {
            self.ranges.push(expr.range());
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}
}

/// The range of every `Dict` literal in `body` carrying more than `cap`
/// entries. A `Dict` inside a replacement field does not count.
fn over_count_dicts(body: &[Stmt], cap: usize) -> Vec<TextRange> {
    let mut scan = DictScan {
        cap,
        ranges: Vec::new(),
    };
    scan.visit_body(body);
    scan.ranges
}

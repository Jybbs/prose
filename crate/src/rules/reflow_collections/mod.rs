//! Lays out `dict`, `list`, `set`, and `tuple` literals against the
//! `Config::code_line_length` budget. A multi-line subscript,
//! comprehension, or dict key whose inline form fits rejoins onto one
//! line, an overflowing single-line literal expands one entry per
//! line, a dict over `max_dict_entries` expands whatever its width,
//! and an over-wide dict entry breaks at `:` and hangs its value. A
//! comment, a replacement field, or a folded multi-line string holds a
//! construct at its source shape, a held member travels with the row
//! it lands on, and `keep_multiline_literals` re-expands an authored
//! flush column rather than joining it. Every measure reads the value
//! at the column `align_equals` shifts it to, the width the padding
//! rule settles it at, and the separator `alphabetize-siblings` leaves
//! closing its row.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        call_keywords::{CallTargets, module_call_params},
        edit::{narrowed_replacement, singleton_groups},
        inline::end_column,
        layout::{is_collapsible, is_layoutable, requires_expand},
        one_row,
        padding::Stranding,
        reserve,
        travel::Landing,
        walk::{Descent, ParentedProbe, filter_map_over_exprs, walk_parented_exprs},
    },
    rule::{Rule, RuleId},
    rules::alphabetize_siblings::Reorders,
    source::Source,
};

mod classify;
mod flow;
mod measure;
mod render;

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

pub(super) const CANONICAL_SEPARATOR: usize = 2;

pub(super) struct Layouter<'a> {
    pub(super) code_line_length: usize,
    pub(super) edits: Vec<Edit>,
    pub(super) explode: bool,
    pub(super) max_atomics: usize,
    pub(super) newline: &'static str,
    pub(super) one_row: one_row::Settings<'a>,
    pub(super) padding: &'a [Edit],
    pub(super) reorders: Reorders,
    pub(super) reservations: &'a reserve::Columns,
    pub(super) source: &'a Source,
    pub(super) targets: &'a CallTargets<'a>,
    pub(super) tripping_dicts: Vec<TextRange>,
    pub(super) wrap_dict_entries: bool,
}

impl<'a> Layouter<'a> {
    /// Returns the canonical rewrite for `expr` under `parent`, or
    /// `None` to descend into its children. `indent` is where the
    /// closing bracket lands on
    /// expand. A multi-line subscript or comprehension that fits rejoins,
    /// while a multi-item `Dict`, `List`, `Set`, or parenthesized `Tuple`
    /// that overflows expands, as does a `Dict` over `max_dict_entries`
    /// and a literal already laid out as a flush column. A subscript and a
    /// comprehension only ever rejoin. The `explode` facet gates every
    /// expansion, and a set `keep_multiline_literals` suppresses the
    /// literal rejoin, a cleared `explode` returning `None`.
    pub(super) fn replacement_for(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        indent: usize,
        tail: usize,
    ) -> Option<String> {
        let range = expr.range();
        if let Some(inline) = self
            .one_row
            .rejoined(self.source, expr, expr.into(), column, tail)
        {
            return Some(inline.into_owned());
        }
        if !is_layoutable(expr) || self.source.intersects_comment(range) {
            return None;
        }
        let expandable = requires_expand(expr);
        let over_count = self.has_over_count_dict(expr);
        if self.source.contains_line_break(range) {
            return (self.explode && expandable).then(|| self.expand(expr, parent, indent));
        }
        (self.explode
            && expandable
            && (over_count
                || column + self.narrowest_width(expr, parent, range) + tail
                    > self.code_line_length))
            .then(|| self.expand(expr, parent, indent))
    }

    /// Serializes `expr` into a child slot of an enclosing expand with
    /// `tail` columns closing its row. Dispatches through
    /// `replacement_for`, falling back to the paren-recovered source
    /// slice placed at `indent` when no rewrite applies. `column` and
    /// `indent` differ for dict values, where the key text sits between
    /// the line indent and the value's own start.
    pub(super) fn serialize_expr(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        indent: usize,
        tail: usize,
    ) -> Cow<'a, str> {
        let landing = Landing {
            column,
            indent,
            item: expr.start(),
        };
        self.replacement_for(expr, parent, column, indent, tail)
            .map_or_else(
                || self.placed_slice(expr, parent, landing, tail),
                Cow::Owned,
            )
    }
}

impl<'a> ParentedProbe<'a> for Layouter<'a> {
    const INTERPOLATIONS: Descent = Descent::Over;

    /// Descends past any expression the rule does not lay out or leaves
    /// as written.
    fn probe(
        &mut self,
        expr: &'a Expr,
        parent: AnyNodeRef<'a>,
        ancestors: &[AnyNodeRef<'a>],
    ) -> Descent {
        if !is_collapsible(expr) {
            return Descent::Into;
        }
        let range = expr.range();
        let start = range.start();
        // Test the collapse against the column `align_equals` shifts the
        // value to and `strip-stranded-padding` settles the row ahead of
        // it at, not the column the literal currently opens at, so a fit
        // that survives both is what the rule collapses. A dict value
        // measures from the canonical `": "` past its key's last row, the column the
        // aligner pads only where the cap allows. Where this walk's own
        // earlier edits rewrote the line ahead of the literal, the column
        // and indent read from the row the literal lands on instead, the
        // column still moved by the shift `align_equals` applies there.
        let (column, indent) = match self.placed_head(start) {
            Cow::Owned(head) => {
                let indent = head.rsplit_once('\n').map_or_else(
                    || self.source.line_indent_width(start),
                    |(_, last)| last.width() - last.trim_start().width(),
                );
                let column = self.reservations.column(start, || end_column(&head, 0));
                (column, indent)
            }
            Cow::Borrowed(_) => {
                let column = dict_key_of(parent, expr).map_or_else(
                    || self.settled_column(start),
                    |key| {
                        end_column(self.source.slice(key), self.source.column_of(key.start()))
                            + CANONICAL_SEPARATOR
                    },
                );
                (column, self.source.line_indent_width(start))
            }
        };
        let grandparent = ancestors[ancestors.len().saturating_sub(2)];
        let tail = self.row_tail(expr, parent, grandparent);
        let Some(text) = self.replacement_for(expr, parent, column, indent, tail) else {
            return Descent::Into;
        };
        self.edits
            .extend(narrowed_replacement(self.source, range, text));
        Descent::Over
    }
}

/// The key of the entry of `parent` whose value is `expr`, `None` for
/// any other expression.
fn dict_key_of<'a>(parent: AnyNodeRef<'a>, expr: &Expr) -> Option<&'a Expr> {
    let AnyNodeRef::ExprDict(dict) = parent else {
        return None;
    };
    dict.items
        .iter()
        .find(|item| item.value.range() == expr.range())?
        .key
        .as_ref()
}

/// The columns closing the entry holding `entry`: `current`, the
/// separator it carries now, where no sort leaves an entry `last`, and
/// otherwise one for an entry the sort leaves anywhere but last. A
/// keyword's value and a dict entry's key or value each read as the
/// entry holding them.
pub(super) fn entry_tail(last: Option<TextRange>, entry: TextRange, current: usize) -> usize {
    last.map_or(current, |last| usize::from(!last.contains_range(entry)))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn a_padded_entry_condensing_inside_the_cap_holds_its_row() {
        let src = "class A:\n    def f(self):\n        self.loggerMap = { alogger : None }\n";
        let source = parse(src);
        let mut config = Config {
            code_line_length: NonZeroUsize::new(40),
            ..Config::default()
        };
        config.rules.strip_stranded_padding.enabled = false;
        assert!(
            ReflowCollections::from_config(&config)
                .apply(&source)
                .is_empty(),
            "the entry measures at the width its rebuild carries, so no expand fires:\n{src}",
        );
    }
}

//! The one-row form of an expression or an argument list, and the
//! terms under which that form exists at all. A rule deciding where a
//! construct lands reads this module, so the decision rests on the
//! shape layout settles on rather than the shape it starts from, and
//! `None` is the answer that counts, in that a flush column
//! `keep_multiline_literals` holds, a dict past `max_dict_entries`,
//! an argument list past `max_args`, a string part spanning rows, and
//! a range carrying a comment each leave a construct with no one-row
//! form whatever its width.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, AnyParameterRef, ArgOrKeyword, Arguments, Expr, ExprCall};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        call_keywords::CallTargets,
        edit::apply_inline_edits,
        fracture::{self, outermost},
        inline::{display_width, settled_slice_width, settled_text_width, spans_rows},
        layout::{
            is_collapse_only, is_collapsible, is_column_shaped, is_multi_entry, requires_expand,
        },
        params::parameter_sites,
        walk::{Interpolations, any_over_expr_within},
    },
    source::Source,
};

mod render;
mod walk;

use render::{Writer, write_joined};

/// The terms a one-row form exists under, resolved from configuration.
/// `rejoin` carries both the argument cap and whether `reflow-calls`
/// closes a fracture at all, `expands_literals` whether
/// `reflow-collections` expands a literal, and `max_dict_entries` is
/// `None` where it does not, leaving the entry cap inert.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings<'a> {
    code_line_length: usize,
    expands_literals: bool,
    keep_multiline_literals: bool,
    max_dict_entries: Option<usize>,
    rejoin: fracture::Settings<'a>,
}

impl<'a> Settings<'a> {
    /// True for a literal the author laid out as a flush column while
    /// `keep_multiline_literals` holds it, which re-expands to that same
    /// column rather than joining.
    fn holds_its_column(&self, source: &Source, expr: &Expr) -> bool {
        self.keep_multiline_literals
            && is_multi_entry(expr.into())
            && is_column_shaped(source.slice(expr.range()))
    }

    /// `expr`'s one-row form when it fits from `column` across `tail`
    /// trailing columns, `hold` deciding whether its own flush column
    /// blocks the form.
    fn measured(
        &self,
        source: &'a Source,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        tail: usize,
        hold: Column,
    ) -> Option<Cow<'a, str>> {
        let range = source.paren_aware_range(expr.into(), parent);
        let form = self.written(source, expr, range, hold)?;
        self.fits(column + display_width(&form) + tail)
            .then_some(form)
    }

    /// The narrower of the width `range` settles to as written and the
    /// width `expr`'s canonical rebuild carries. `padding` is the edit
    /// list `strip-stranded-padding` emits, discounted from the
    /// as-written reading alone.
    fn narrowest_width(
        &self,
        source: &Source,
        expr: &Expr,
        parent: AnyNodeRef,
        range: TextRange,
        padding: &[Edit],
    ) -> usize {
        let settled = settled_slice_width(source, padding, range);
        let condensed = self
            .condensed(source, expr, parent)
            .map_or(settled, |text| {
                settled_text_width(source, padding, &text, range)
            });
        settled.min(condensed)
    }

    /// The writer serializing under these settings over `source`.
    fn writer(&self, source: &'a Source) -> Writer<'a> {
        Writer {
            settings: *self,
            source,
        }
    }

    /// `expr`'s one-row form over `range`, `hold` deciding whether its
    /// own flush column blocks the form.
    fn written(
        &self,
        source: &'a Source,
        expr: &Expr,
        range: TextRange,
        hold: Column,
    ) -> Option<Cow<'a, str>> {
        self.writer(source).formed(expr, range, hold)
    }

    /// These settings resolving each call against `targets`, the map
    /// [`module_call_params`](crate::primitives::call_keywords::module_call_params)
    /// builds for one source.
    pub(crate) fn against<'t>(self, targets: &'t CallTargets<'t>) -> Settings<'t> {
        Settings {
            code_line_length: self.code_line_length,
            expands_literals: self.expands_literals,
            keep_multiline_literals: self.keep_multiline_literals,
            max_dict_entries: self.max_dict_entries,
            rejoin: self.rejoin.against(targets),
        }
    }

    /// The one-row `(...)` form of `arguments`, `None` where no one-row
    /// form exists. A single-row argument sheds a redundant grouping
    /// pair and a row-spanning one keeps the pair holding its rows
    /// together. This list's own argument count is left to the caller's
    /// count trigger, whereas an argument holding a construct a later
    /// rule lays out across rows reaches no form at all.
    pub(crate) fn arguments_form(
        &self,
        source: &'a Source,
        arguments: &Arguments,
    ) -> Option<String> {
        let writer = self.writer(source);
        if source.intersects_comment(arguments.inner_range()) {
            return None;
        }
        let mut out = String::from("(");
        write_joined(
            &mut out,
            arguments.iter_source_order(),
            |out, arg| match arg {
                ArgOrKeyword::Arg(expr) => writer.write_argument(out, expr, arguments.into()),
                ArgOrKeyword::Keyword(kw) => {
                    match &kw.arg {
                        Some(name) => {
                            out.push_str(name);
                            out.push('=');
                        }
                        None => out.push_str("**"),
                    }
                    writer.write_argument(out, &kw.value, kw.into())
                }
            },
        )?;
        out.push(')');
        Some(out)
    }

    /// `expr`'s one-row form rebuilt at the canonical spacing, whatever
    /// padding the source wrote inside it. `None` where no one-row form
    /// exists.
    pub(crate) fn condensed(
        &self,
        source: &'a Source,
        expr: &Expr,
        parent: AnyNodeRef,
    ) -> Option<Cow<'a, str>> {
        let range = source.paren_aware_range(expr.into(), parent);
        self.writer(source).condensed(expr, range, Column::Holds)
    }

    /// True where `reflow-calls`'s count trigger explodes `call`, read
    /// off the rejoin terms these settings carry.
    pub(crate) fn count_explodes(&self, source: &Source, call: &ExprCall) -> bool {
        self.rejoin.explodes(source, call)
    }

    /// True where `reflow-collections` expands `literal` at `column` with
    /// `tail` columns after it. A comment-free literal passing
    /// `requires_expand` expands where a later rule reopens it, where it
    /// is written across rows, or where its narrowest width under
    /// `padding` overflows. A caller tries [`Self::rejoined`] first.
    pub(crate) fn expands(
        &self,
        source: &'a Source,
        literal: &Expr,
        parent: AnyNodeRef,
        column: usize,
        tail: usize,
        padding: &[Edit],
    ) -> bool {
        let range = literal.range();
        self.expands_literals
            && requires_expand(literal)
            && !source.intersects_comment(range)
            && (source.contains_line_break(range)
                || self.reopens(source, literal)
                || !self.fits(
                    column + self.narrowest_width(source, literal, parent, range, padding) + tail,
                ))
    }

    /// True where `reflow-collections` expands literals, meaning the rule
    /// is on and its `explode` facet is set.
    pub(crate) fn expands_literals(&self) -> bool {
        self.expands_literals
    }

    /// True where a row reaching `width` columns sits inside the budget.
    pub(crate) fn fits(&self, width: usize) -> bool {
        width <= self.code_line_length
    }

    /// `expr`'s one-row form measured from `column` with `tail` columns
    /// of text following it, `None` where no one-row form exists or the
    /// row it lands on overflows the budget.
    pub(crate) fn fitted(
        &self,
        source: &'a Source,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        tail: usize,
    ) -> Option<Cow<'a, str>> {
        self.measured(source, expr, parent, column, tail, Column::Holds)
    }

    /// `param`'s one-row text, each row-spanning annotation and default
    /// spliced back at its own one-row form, keeping the spacing the
    /// source wrote around `:` and `=`. `None` where either reaches no
    /// single row.
    pub(crate) fn parameter_form(
        &self,
        source: &'a Source,
        param: AnyParameterRef,
    ) -> Option<String> {
        let range = param.range();
        if source.intersects_comment(range) {
            return None;
        }
        let joins = parameter_sites(param)
            .into_iter()
            .filter(|(expr, _)| source.contains_line_break(expr.range()))
            .map(|(expr, parent)| {
                let held = source.paren_aware_range(expr.into(), parent);
                let form = self.written(source, expr, held, Column::Holds)?;
                Some(Edit::range_replacement(form.into_owned(), held))
            })
            .collect::<Option<Vec<_>>>()?;
        let text = apply_inline_edits(source, range, &outermost(joins));
        (!spans_rows(&text)).then(|| text.into_owned())
    }

    /// `expr`'s one-row form where the layout rules rejoin it onto its
    /// row, meaning a collection literal written across lines that fits
    /// from `column` across `tail` trailing columns, or a subscript or
    /// comprehension whose repair fits, each free of comments. `None`
    /// for any other expression and wherever the form overflows.
    pub(crate) fn rejoined(
        &self,
        source: &'a Source,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        tail: usize,
    ) -> Option<Cow<'a, str>> {
        let range = expr.range();
        if !is_collapsible(expr)
            || !source.comment_ranges().comments_in_range(range).is_empty()
            || !source.contains_line_break(range)
        {
            return None;
        }
        if is_collapse_only(expr) {
            self.repaired(source, expr, parent, column, tail)
        } else {
            self.fitted(source, expr, parent, column, tail)
        }
    }

    /// True where a later rule reopens `expr` whatever its shape. That
    /// happens where `expr` holds a dict past `max_dict_entries`, or a
    /// call past `max_args` that `reflow-calls` can name, outside any
    /// replacement field.
    pub(crate) fn reopens(&self, source: &Source, expr: &Expr) -> bool {
        any_over_expr_within(expr, Interpolations::Skip, |e| {
            e.as_call_expr()
                .is_some_and(|call| self.rejoin.explodes(source, call))
                || e.as_dict_expr()
                    .is_some_and(|dict| self.max_dict_entries.is_some_and(|cap| dict.len() > cap))
        })
    }

    /// `expr`'s one-row form measured from `column` across `tail`
    /// trailing columns, its own flush column joining rather than
    /// holding. This is the reading a construct takes whose break falls
    /// outside the entry boundaries the expand path lays a literal out
    /// on, meaning a dict key, a subscript index, and a comprehension. A
    /// flush column nested inside it still holds, that one being an
    /// entry boundary.
    pub(crate) fn repaired(
        &self,
        source: &'a Source,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        tail: usize,
    ) -> Option<Cow<'a, str>> {
        self.measured(source, expr, parent, column, tail, Column::Joins)
    }
}

impl From<&Config> for Settings<'_> {
    fn from(config: &Config) -> Self {
        let collection = &config.rules.reflow_collections;
        Self {
            code_line_length: config.code_width(),
            expands_literals: config.expands_literals(),
            keep_multiline_literals: collection.keep_multiline_literals,
            max_dict_entries: collection
                .max_dict_entries
                .cap()
                .filter(|_| config.expands_literals()),
            rejoin: config.fracture_settings(),
        }
    }
}

/// Whether a construct's own flush column blocks its one-row form.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Column {
    /// The column holds, the reading every construct but the three
    /// below takes.
    Holds,
    /// The column joins, the reading a dict key, a subscript index, and
    /// a comprehension take.
    Joins,
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

    /// `src`'s first expression's one-row form under `config`.
    fn form_under(config: &Config, src: &str) -> Option<String> {
        let source = parse(src);
        let expr = first_expr(&source);
        Settings::from(config)
            .fitted(&source, expr, expr.into(), 0, 0)
            .map(Cow::into_owned)
    }

    #[rstest]
    #[case::fracture_closes("helper(a,\n       b)\n", Some("(a, b)"))]
    #[case::single_row_grouping_pair_drops("helper((a),\n       b)\n", Some("(a, b)"))]
    #[case::own_count_left_to_the_caller("helper(a, b, c, d)\n", Some("(a, b, c, d)"))]
    #[case::nested_list_past_the_cap("helper(inner(a, b, c, d))\n", None)]
    #[case::held_column_argument("helper([\n    a,\n    b,\n])\n", None)]
    fn arguments_form_answers_the_list_the_call_lands_on(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let call = first_expr(&source).as_call_expr().expect("a call");
        assert_eq!(
            Settings::from(&Config::default())
                .arguments_form(&source, &call.arguments)
                .as_deref(),
            expected,
        );
    }

    #[rstest]
    #[case::fits_its_row("[a, b]", 0, 0, false)]
    #[case::overflows_through_its_tail("[a, b]", 0, 85, true)]
    #[case::overflows_from_its_column("[a, b]", 84, 0, true)]
    #[case::one_entry_dict_overflowing("{'k': v}", 85, 0, true)]
    #[case::one_element_list("[aaaa]", 90, 0, false)]
    #[case::comment_inside("[\n    a,  # c\n    b,\n]", 90, 0, false)]
    #[case::written_across_rows("[\n    a,\n    b,\n]", 0, 0, true)]
    #[case::count_exploded_call_inside("[helper(a=1, b=2, c=3, d=4), b]", 0, 0, true)]
    fn expands_reads_each_trigger_reflow_collections_expands_on(
        #[case] src: &str,
        #[case] column: usize,
        #[case] tail: usize,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let literal = first_expr(&source);
        assert_eq!(
            Settings::from(&Config::default()).expands(
                &source,
                literal,
                literal.into(),
                column,
                tail,
                &[],
            ),
            expected,
        );
    }

    #[rstest]
    #[case::fits(0, 0, true)]
    #[case::tail_overflows(0, 84, false)]
    #[case::column_overflows(84, 0, false)]
    fn fitted_charges_the_column_and_the_trailing_text(
        #[case] column: usize,
        #[case] tail: usize,
        #[case] fits: bool,
    ) {
        let source = parse("[a, b]");
        let expr = first_expr(&source);
        assert_eq!(
            Settings::from(&Config::default())
                .fitted(&source, expr, expr.into(), column, tail)
                .is_some(),
            fits,
        );
    }

    #[rstest]
    #[case::already_flat("[a, b]", Some("[a, b]"))]
    #[case::fracture_closes("[\n    a, b]", Some("[a, b]"))]
    #[case::nested_literal_joins("{\n    'k': [\n        1, 2]}", Some("{'k': [1, 2]}"))]
    #[case::subscript_joins("table[\n    key]", Some("table[key]"))]
    #[case::held_column("[\n    a,\n    b,\n]", None)]
    #[case::multiline_string("[\n    \"\"\"x\ny\"\"\"]", None)]
    #[case::comment_inside("[\n    a,  # note\n    b]", None)]
    #[case::over_cap_call_inside("[helper(a, b, c, d)]", None)]
    fn form_answers_none_where_no_one_row_shape_survives(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(form_under(&Config::default(), src).as_deref(), expected);
    }

    #[test]
    fn form_declines_a_dict_past_the_entry_cap() {
        let mut config = Config::default();
        config.rules.reflow_collections.max_dict_entries.0 = NonZeroUsize::new(2);
        assert_eq!(form_under(&config, "{'a': 1, 'b': 2, 'c': 3}"), None);
    }

    #[rstest]
    #[case::explode_facet_cleared(true, false)]
    #[case::rule_disabled(false, true)]
    fn form_joins_a_dict_the_entry_cap_leaves_inert(#[case] enabled: bool, #[case] explode: bool) {
        let mut config = Config::default();
        config.rules.reflow_collections.enabled = enabled;
        config.rules.reflow_collections.explode = explode;
        config.rules.reflow_collections.max_dict_entries.0 = NonZeroUsize::new(2);
        assert_eq!(
            form_under(&config, "{'a': 1,\n 'b': 2, 'c': 3}").as_deref(),
            Some("{'a': 1, 'b': 2, 'c': 3}"),
        );
    }

    #[rstest]
    #[case::call_past_the_argument_cap("[helper(a=1, b=2, c=3, d=4)]", true)]
    #[case::dict_past_the_entry_cap("[{'a': 1, 'b': 2, 'c': 3, 'd': 4}]", true)]
    #[case::call_at_the_argument_cap("[helper(a=1, b=2, c=3)]", false)]
    #[case::call_inside_a_replacement_field("[f\"{helper(a=1, b=2, c=3, d=4)}\"]", false)]
    #[case::dict_inside_a_replacement_field("[f\"{ {'a': 1, 'b': 2, 'c': 3, 'd': 4} }\"]", false)]
    fn reopens_reads_the_constructs_a_later_rule_explodes(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(
            Settings::from(&Config::default()).reopens(&source, first_expr(&source)),
            expected,
        );
    }
}

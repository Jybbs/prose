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
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    primitives::{
        call_keywords::CallTargets,
        edit::apply_inline_edits,
        fracture::{self, outermost},
        inline::{display_width, settled_slice_width, settled_width, spans_rows},
        layout::{is_collapse_only, is_collapsible, is_column_shaped, is_multi_entry},
        params::parameter_sites,
        slots::{holds_exactly, item_covering, item_holding, starting_within},
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
/// `reflow-collections` expands a literal, `max_dict_entries` is `None`
/// where it does not, leaving the entry cap inert, and `rewrites` holds
/// the f-string rewrites a form is measured through, none until
/// [`forecasting`](Self::forecasting) binds one source's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings<'a> {
    code_line_length: usize,
    expands_literals: bool,
    keep_multiline_literals: bool,
    max_dict_entries: Option<usize>,
    rejoin: fracture::Settings<'a>,
    rewrites: &'a [Edit],
}

impl<'a> Settings<'a> {
    /// True where [`Self::expands`] expands `literal` once it is written
    /// across rows, meaning `reflow-collections` expands literals and
    /// [`Source::is_expandable`] accepts `literal`.
    fn expands_across_rows(&self, source: &Source, literal: &Expr) -> bool {
        self.expands_literals && source.is_expandable(literal)
    }

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
        self.fits(column + self.form_width(source, &form, range) + tail)
            .then_some(form)
    }

    /// The narrower of the width `range` settles to as written and the
    /// width `expr`'s canonical rebuild carries. `padding` is the edit
    /// list `strip-stranded-padding` emits merged with the forecast
    /// rewrites, discounted from the as-written reading, whereas the
    /// rebuild carries no padding and takes off the rewrites alone.
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
                self.text_width(source, padding, &text, range)
            });
        settled.min(condensed)
    }

    /// Measures [`Self::row_tail`] past `end`, keeping in `memo` the tail
    /// it reads past each later literal, keyed by that literal's end.
    fn tail_through(
        &self,
        source: &Source,
        padding: &[Edit],
        end: TextSize,
        memo: &mut FxHashMap<TextSize, usize>,
    ) -> usize {
        let mut tail = source.row_tail(end);
        if self.expands_literals {
            let landing = source.line_indent_width(end) + 1;
            let mut covered = end;
            for &literal in starting_within(source.expandable_literals(), tail, Ranged::start) {
                if literal.start() < covered || self.rewritten(literal.start()) {
                    continue;
                }
                covered = literal.end();
                let expands = source.contains_line_break(literal) || {
                    let own = if let Some(&own) = memo.get(&literal.end()) {
                        own
                    } else {
                        let own = self.tail_through(source, padding, literal.end(), memo);
                        memo.insert(literal.end(), own);
                        own
                    };
                    !self.fits(
                        landing
                            + settled_slice_width(
                                source,
                                padding,
                                TextRange::new(end, literal.start()),
                            )
                            + settled_slice_width(source, padding, literal)
                            + own,
                    )
                };
                if expands {
                    tail = TextRange::new(end, literal.start() + TextSize::from(1));
                    break;
                }
            }
        }
        settled_width(source, padding, tail, source.tail_width(tail))
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
    pub(crate) fn against<'t>(self, targets: &'t CallTargets<'t>) -> Settings<'t>
    where
        'a: 't,
    {
        Settings {
            code_line_length: self.code_line_length,
            expands_literals: self.expands_literals,
            keep_multiline_literals: self.keep_multiline_literals,
            max_dict_entries: self.max_dict_entries,
            rejoin: self.rejoin.against(targets),
            rewrites: self.rewrites,
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

    /// True where `reflow-calls` is enabled, read off the rejoin terms
    /// these settings carry.
    pub(crate) fn closes(&self) -> bool {
        self.rejoin.closes()
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
    /// `tail` columns after it. A literal [`Source::is_expandable`] accepts
    /// expands where a later rule reopens it, where it is written across
    /// rows, or where its narrowest width under `padding` overflows. A
    /// caller tries [`Self::rejoined`] first.
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
        self.expands_across_rows(source, literal)
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

    /// True where `reflow-calls` runs and [`Source::explodable_arguments`]
    /// lists `arguments`.
    pub(crate) fn explodes_arguments(&self, source: &Source, arguments: &Arguments) -> bool {
        self.closes() && holds_exactly(source.explodable_arguments(), arguments.range())
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

    /// These settings measuring each one-row form through `rewrites`,
    /// the f-string rewrites [`Source::fstring_rewrites`] forecasts for
    /// the source the form is written over.
    pub(crate) fn forecasting(self, rewrites: &'a [Edit]) -> Self {
        Self { rewrites, ..self }
    }

    /// The display width of `form`, a one-row form written over `range`,
    /// once each forecast rewrite inside `range` lands.
    pub(crate) fn form_width(&self, source: &Source, form: &str, range: TextRange) -> usize {
        settled_width(source, self.rewrites, range, display_width(form))
    }

    /// True for a literal written on one row that [`Self::expands`]
    /// expands once written across rows. A walk that leaves such a
    /// literal as written leaves every literal inside it on that row.
    pub(crate) fn holds_its_row(&self, source: &Source, literal: &Expr) -> bool {
        !source.contains_line_break(literal.range()) && self.expands_across_rows(source, literal)
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

    /// The forecast rewrite whose replaced text covers `range`, `None`
    /// where no rewrite covers it.
    pub(crate) fn rewrite_covering(&self, range: TextRange) -> Option<&'a Edit> {
        item_covering(self.rewrites, range)
    }

    /// True where a forecast rewrite replaces the text at `offset`.
    pub(crate) fn rewritten(&self, offset: TextSize) -> bool {
        item_holding(self.rewrites, offset).is_some_and(|rewrite| rewrite.range().contains(offset))
    }

    /// The columns trailing `end` on its row once `padding` settles
    /// them, closing just past the opening bracket of the first later
    /// literal on the row that expands anyway. Such a literal is written
    /// across rows or overflows from where it lands once the construct
    /// closing at `end` breaks and drops its closer to the row's indent,
    /// measured with its own tail read the same way. A literal inside a
    /// forecast rewrite holds its row.
    pub(crate) fn row_tail(&self, source: &Source, padding: &[Edit], end: TextSize) -> usize {
        self.tail_through(source, padding, end, &mut FxHashMap::default())
    }

    /// The display width of the source slice over `range` once each
    /// forecast rewrite inside it lands.
    pub(crate) fn slice_width(&self, source: &Source, range: TextRange) -> usize {
        self.form_width(source, source.slice(range), range)
    }

    /// The display width `text` settles to over `range`, the settled
    /// width of `range` under `padding` where `text` is that source slice
    /// as written, and the [`form_width`](Self::form_width) of a rewrite,
    /// which carries no padding.
    pub(crate) fn text_width(
        &self,
        source: &Source,
        padding: &[Edit],
        text: &str,
        range: TextRange,
    ) -> usize {
        if source.slice(range) == text {
            settled_slice_width(source, padding, range)
        } else {
            self.form_width(source, text, range)
        }
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
            rewrites: &[],
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
    use ruff_python_ast::PythonVersion;

    use super::*;
    use crate::testing::{first_expr, first_value, parse};

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
    #[case::as_written(false, None)]
    #[case::through_the_forecast(true, Some("[\"%s\" % (a,)]"))]
    fn fitted_measures_a_forecast_rewrite_at_its_fstring_width(
        #[case] forecast: bool,
        #[case] expected: Option<&str>,
    ) {
        let config = Config {
            code_line_length: NonZeroUsize::new(12),
            target_version: Some(PythonVersion::PY310),
            ..Config::default()
        };
        let source = parse("[\n    \"%s\" % (a,)]");
        let expr = first_expr(&source);
        let rewrites = if forecast {
            config.fstrings().forecast(&source)
        } else {
            Vec::new()
        };
        assert_eq!(
            Settings::from(&config)
                .forecasting(&rewrites)
                .fitted(&source, expr, expr.into(), 0, 0)
                .as_deref(),
            expected,
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
    #[case::one_row_list("[a, b]", true, true)]
    #[case::explode_facet_cleared("[a, b]", false, false)]
    #[case::written_across_rows("[\n    a,\n    b,\n]", true, false)]
    #[case::one_element_list("[aaaa]", true, false)]
    fn holds_its_row_reads_a_one_row_literal_the_rule_expands(
        #[case] src: &str,
        #[case] explode: bool,
        #[case] expected: bool,
    ) {
        let mut config = Config::default();
        config.rules.reflow_collections.explode = explode;
        let source = parse(src);
        assert_eq!(
            Settings::from(&config).holds_its_row(&source, first_expr(&source)),
            expected,
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

    #[rstest]
    #[case::literals_never_expand(false, false)]
    #[case::literal_inside_a_forecast_rewrite(true, true)]
    fn row_tail_charges_a_literal_that_holds_its_row_whole(
        #[case] explode: bool,
        #[case] rewritten: bool,
    ) {
        let src = "x = [a, b] + [bbbbbbbbbb, cccccccccc]\n";
        let mut config = Config {
            code_line_length: NonZeroUsize::new(20),
            ..Config::default()
        };
        config.rules.reflow_collections.explode = explode;
        let source = parse(src);
        let operands = first_value(&source)
            .as_bin_op_expr()
            .expect("the value adds two literals");
        let rewrites: Vec<Edit> = rewritten
            .then(|| Edit::range_replacement("f\"\"".to_owned(), operands.right.range()))
            .into_iter()
            .collect();
        let settings = Settings::from(&config).forecasting(&rewrites);
        assert_eq!(
            settings.row_tail(&source, &[], operands.left.end()),
            " + [bbbbbbbbbb, cccccccccc]".len(),
        );
    }

    #[rstest]
    #[case::later_literal_fits("x = [a, b] + [c, d]\n", 40, " + [c, d]")]
    #[case::later_literal_fits_once_the_earlier_breaks(
        "x = [aaaaaaaaaaaaaaa, b] + [bbbbbbbb, cc]\n",
        20,
        " + [bbbbbbbb, cc]"
    )]
    #[case::later_literal_expands_anyway("x = [a, b] + [bbbbbbbbbb, cccccccccc]\n", 20, " + [")]
    #[case::later_literal_written_across_rows("x = [a, b] + [\n    c,\n    d,\n]\n", 88, " + [")]
    fn row_tail_closes_at_a_later_literal_that_expands_anyway(
        #[case] src: &str,
        #[case] width: usize,
        #[case] tail: &str,
    ) {
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            ..Config::default()
        };
        let source = parse(src);
        let first = &first_value(&source)
            .as_bin_op_expr()
            .expect("the value adds two literals")
            .left;
        assert_eq!(
            Settings::from(&config).row_tail(&source, &[], first.end()),
            tail.len(),
        );
    }
}

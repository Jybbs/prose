//! The one-row form of an expression or an argument list, and the
//! terms under which that form exists at all. A rule deciding where a
//! construct lands asks this module rather than reading the breaks the
//! author happened to write, so the decision rests on the shape layout
//! settles on rather than on the shape it starts from.
//!
//! `None` is the answer that carries the weight. A flush column
//! `keep_multiline_literals` holds, a dict past `max_dict_entries`, an
//! argument list past `max_args`, a string part spanning rows, and a
//! range carrying a comment each leave a construct with no one-row form
//! whatever its width, where a caller measuring width alone reads a
//! first row and takes it for the whole.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, ArgOrKeyword, Arguments, Comprehension, Expr, ExprCall, ExprDict,
    helpers::any_over_expr,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        call_keywords::CallTargets,
        edit::apply_inline_edits,
        fracture::{self, outermost},
        inline::folded_line_form,
        layout::{is_collapsible, is_column_shaped, is_fractured, is_multi_entry},
    },
    source::Source,
};

/// The terms a one-row form exists under, resolved from configuration.
/// `rejoin` carries both the argument cap and whether `reflow-calls`
/// closes a fracture at all, and `max_dict_entries` is `None` where the
/// `explode` facet leaves the entry cap inert.
#[derive(Clone, Copy)]
pub(crate) struct Settings<'a> {
    code_line_length: usize,
    keep_multiline_literals: bool,
    max_dict_entries: Option<usize>,
    rejoin: fracture::Settings<'a>,
}

impl<'a> Settings<'a> {
    /// The one-row `(...)` form of `arguments`, `None` where no one-row
    /// form exists. A single-row argument sheds a redundant grouping
    /// pair, which a top-level argument slot never needs, and a
    /// row-spanning one keeps the pair holding its rows together. This
    /// list's own argument count is left to the caller's count trigger,
    /// whereas an argument holding a construct a later rule lays out
    /// across rows reaches no form at all.
    pub(crate) fn arguments_form(
        &self,
        source: &'a Source,
        arguments: &Arguments,
    ) -> Option<String> {
        let writer = Writer {
            settings: *self,
            source,
        };
        if source.intersects_comment(arguments.inner_range()) {
            return None;
        }
        let mut out = String::from("(");
        for (i, arg) in arguments.iter_source_order().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            let written = match arg {
                ArgOrKeyword::Arg(expr) => writer.write_argument(&mut out, expr, arguments.into()),
                ArgOrKeyword::Keyword(kw) => {
                    match &kw.arg {
                        Some(name) => {
                            out.push_str(name);
                            out.push('=');
                        }
                        None => out.push_str("**"),
                    }
                    writer.write_argument(&mut out, &kw.value, kw.into())
                }
            };
            written?;
        }
        out.push(')');
        Some(out)
    }

    /// These settings resolving a call against `targets`, the map
    /// [`module_call_params`](crate::primitives::call_keywords::module_call_params)
    /// builds for one source. A rule reads the count trigger the same
    /// way `reflow-calls` does once it carries the map.
    pub(crate) fn against<'t>(self, targets: &'t CallTargets<'t>) -> Settings<'t> {
        Settings {
            code_line_length: self.code_line_length,
            keep_multiline_literals: self.keep_multiline_literals,
            max_dict_entries: self.max_dict_entries,
            rejoin: self.rejoin.against(targets),
        }
    }

    /// True where `reflow-calls`'s count trigger explodes `call`, read
    /// off the rejoin terms these settings carry.
    pub(crate) fn count_explodes(&self, source: &Source, call: &ExprCall) -> bool {
        self.rejoin.explodes(source, call)
    }

    /// The dict entry cap the `explode` facet leaves armed, `None` where
    /// no count expands a dict.
    pub(crate) fn dict_entry_cap(&self) -> Option<usize> {
        self.max_dict_entries
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
    /// spliced back at its own one-row form so the spacing the source
    /// wrote around `:` and `=` survives, `None` where either reaches no
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
        let inner = param.as_parameter();
        let mut sites: Vec<(&Expr, AnyNodeRef)> = Vec::new();
        if let Some(annotation) = inner.annotation.as_deref() {
            sites.push((annotation, inner.into()));
        }
        if let AnyParameterRef::NonVariadic(slot) = param
            && let Some(default) = slot.default.as_deref()
        {
            sites.push((default, slot.into()));
        }
        let mut joins = Vec::new();
        for (expr, parent) in sites {
            if !source.contains_line_break(expr.range()) {
                continue;
            }
            let held = source.paren_aware_range(expr.into(), parent);
            let form = self.written(source, expr, held, Column::Holds)?;
            joins.push(Edit::range_replacement(form.into_owned(), held));
        }
        let text = apply_inline_edits(source, range, &outermost(joins));
        (!text.contains('\n')).then(|| text.into_owned())
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
        self.fits(column + form.width() + tail).then_some(form)
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
        let writer = Writer {
            settings: *self,
            source,
        };
        writer.formed(expr, range, hold)
    }

    /// True for a literal the author laid out as a flush column while
    /// `keep_multiline_literals` holds it, which re-expands to that same
    /// column rather than joining.
    fn holds_its_column(&self, source: &Source, expr: &Expr) -> bool {
        self.keep_multiline_literals
            && is_multi_entry(expr.into())
            && is_column_shaped(source.slice(expr.range()))
    }
}

impl From<&Config> for Settings<'_> {
    fn from(config: &Config) -> Self {
        let collection = &config.rules.reflow_collections;
        Self {
            code_line_length: config.code_width(),
            keep_multiline_literals: collection.keep_multiline_literals,
            max_dict_entries: collection
                .max_dict_entries
                .cap()
                .filter(|_| collection.explode),
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

/// Collects the join closing each fractured bracketed construct beneath
/// one leaf expression, a call's argument list taking its one-row
/// `(...)` form and a literal, subscript, or comprehension its own.
/// `reachable` clears where any of them reaches no single row, leaving
/// the whole leaf without a form.
struct Joiner<'a, 'w> {
    edits: Vec<Edit>,
    reachable: bool,
    writer: &'w Writer<'a>,
}

impl Joiner<'_, '_> {
    /// The one-row text closing `range`, `None` where none exists. An
    /// argument list already carrying the flush column shape the explode
    /// path emits holds that shape, the same reading `is_fractured`
    /// gives a list a join could close.
    fn closed(&self, expr: &Expr, range: TextRange) -> Option<String> {
        let source = self.writer.source;
        let Expr::Call(call) = expr else {
            return self
                .writer
                .formed(expr, range, Column::Holds)
                .map(Cow::into_owned);
        };
        is_fractured(source, range)
            .then(|| self.writer.settings.arguments_form(source, &call.arguments))
            .flatten()
    }

    /// The range a fractured bracketed construct at `expr` closes over,
    /// `None` where `expr` carries no such construct or holds no break.
    fn fractured(&self, expr: &Expr) -> Option<TextRange> {
        let range = match expr {
            Expr::Call(call) => call.arguments.range(),
            _ if is_collapsible(expr) => expr.range(),
            _ => return None,
        };
        self.writer
            .source
            .contains_line_break(range)
            .then_some(range)
    }
}

impl<'ast> AstVisitor<'ast> for Joiner<'_, '_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if !self.reachable {
            return;
        }
        walk_expr(self, expr);
        let Some(range) = self.fractured(expr) else {
            return;
        };
        match self.closed(expr, range) {
            Some(form) => self.edits.push(Edit::range_replacement(form, range)),
            None => self.reachable = false,
        }
    }
}

/// Serializes an expression tree onto one row, each method writing into
/// the caller's buffer and answering `None` where its subtree reaches no
/// one-row form.
struct Writer<'a> {
    settings: Settings<'a>,
    source: &'a Source,
}

impl<'a> Writer<'a> {
    /// `expr`'s one-row form over `range`, borrowing the source slice
    /// where that range is already written flat and rebuilding it from
    /// the children otherwise. `hold` reaches `expr` itself, every child
    /// beneath it holding its own flush column either way.
    fn formed(&self, expr: &Expr, range: TextRange, hold: Column) -> Option<Cow<'a, str>> {
        if (hold == Column::Holds && self.settings.holds_its_column(self.source, expr))
            || self.source.intersects_comment(range)
            || self.reopens(expr)
        {
            return None;
        }
        let slice = self.source.slice(range);
        if !slice.contains('\n') {
            return Some(Cow::Borrowed(slice));
        }
        let mut out = String::new();
        self.write(&mut out, expr, expr.into())?;
        (!out.contains('\n')).then_some(Cow::Owned(out))
    }

    /// The one-row form of a leaf, meaning an expression the dispatch in
    /// [`Self::write`] does not itself rebuild. An operator tree over
    /// atoms soft-wrapped across rows folds its break, and every other
    /// leaf reaches one row by closing each bracketed construct beneath
    /// it, whether that is a call's argument list or a literal the
    /// author fractured. A break no close reaches leaves `None`.
    fn leaf_form(&self, expr: &Expr, range: TextRange) -> Option<Cow<'a, str>> {
        if let Some(folded) = folded_line_form(expr, self.source.slice(range)) {
            return Some(folded);
        }
        let mut joiner = Joiner {
            edits: Vec::new(),
            reachable: true,
            writer: self,
        };
        joiner.visit_expr(expr);
        if !joiner.reachable {
            return None;
        }
        let joined = apply_inline_edits(self.source, range, &outermost(joiner.edits));
        (!joined.contains('\n')).then_some(joined)
    }

    /// True where a later rule reopens `expr` whatever its current
    /// shape. A dict past `max_dict_entries` explodes on its own count
    /// trigger, and so does an argument list past `max_args` that
    /// `reflow-calls` can name, so no one-row form written around either
    /// survives the pipeline. A call the count trigger claims but cannot
    /// rewrite into keyword form stays inline, leaving its one-row form
    /// standing.
    fn reopens(&self, expr: &Expr) -> bool {
        any_over_expr(expr, |e| {
            e.as_call_expr()
                .is_some_and(|call| self.settings.rejoin.explodes(self.source, call))
                || e.as_dict_expr().is_some_and(|dict| {
                    self.settings
                        .max_dict_entries
                        .is_some_and(|cap| dict.len() > cap)
                })
        })
    }

    /// Appends `expr`'s one-row serialization to `out`, dispatching on
    /// its shape and descending through [`Self::write_child`]. `parent`
    /// is the immediate enclosing AST node, read for paren recovery on
    /// non-collection leaves.
    fn write(&self, out: &mut String, expr: &Expr, parent: AnyNodeRef) -> Option<()> {
        let here = AnyNodeRef::from(expr);
        match expr {
            Expr::Dict(d) => self.write_dict(out, d, here),
            Expr::DictComp(c) => self.write_comprehension(
                out,
                Some(('{', '}')),
                c.key.as_deref(),
                &c.value,
                &c.generators,
                here,
            ),
            Expr::Generator(c) => {
                let brackets = c.parenthesized.then_some(('(', ')'));
                self.write_comprehension(out, brackets, None, &c.elt, &c.generators, here)
            }
            Expr::List(l) => self.write_seq(out, Some(('[', ']')), &l.elts, here, false),
            Expr::ListComp(c) => {
                self.write_comprehension(out, Some(('[', ']')), None, &c.elt, &c.generators, here)
            }
            Expr::Set(s) => self.write_seq(out, Some(('{', '}')), &s.elts, here, false),
            Expr::SetComp(c) => {
                self.write_comprehension(out, Some(('{', '}')), None, &c.elt, &c.generators, here)
            }
            Expr::Subscript(s) => {
                self.write_child(out, &s.value, here)?;
                out.push('[');
                self.write_child(out, &s.slice, here)?;
                out.push(']');
                Some(())
            }
            Expr::Tuple(t) => {
                let brackets = t.parenthesized.then_some(('(', ')'));
                self.write_seq(out, brackets, &t.elts, here, t.len() == 1)
            }
            _ => {
                let range = self.source.paren_aware_range(expr.into(), parent);
                out.push_str(&self.leaf_form(expr, range)?);
                Some(())
            }
        }
    }

    /// Appends one top-level call argument to `out`, its grouping parens
    /// recovered only where its own text spans rows.
    fn write_argument(&self, out: &mut String, expr: &Expr, parent: AnyNodeRef) -> Option<()> {
        let range = self.source.spanning_paren_range(expr.into(), parent);
        out.push_str(&self.formed(expr, range, Column::Holds)?);
        Some(())
    }

    /// Appends a child `expr`'s one-row serialization to `out`, a held
    /// flush column leaving the enclosing form unreachable.
    fn write_child(&self, out: &mut String, expr: &Expr, parent: AnyNodeRef) -> Option<()> {
        if self.settings.holds_its_column(self.source, expr) {
            return None;
        }
        self.write(out, expr, parent)
    }

    /// Appends a comprehension's bracketed one-row form to `out`: an
    /// optional `key: ` head, the element, then the clause chain,
    /// wrapped in `brackets`. A `None` bracket carries the bare
    /// generator whose call parens stand in, a `Some` key the dict
    /// comprehension's head.
    fn write_comprehension(
        &self,
        out: &mut String,
        brackets: Option<(char, char)>,
        key: Option<&Expr>,
        element: &Expr,
        generators: &[Comprehension],
        parent: AnyNodeRef,
    ) -> Option<()> {
        let (open, close) = brackets.unzip();
        out.extend(open);
        if let Some(key) = key {
            self.write_child(out, key, parent)?;
            out.push_str(": ");
        }
        self.write_child(out, element, parent)?;
        for generator in generators {
            out.push_str(if generator.is_async {
                " async for "
            } else {
                " for "
            });
            self.write_child(out, &generator.target, parent)?;
            out.push_str(" in ");
            self.write_child(out, &generator.iter, parent)?;
            for condition in &generator.ifs {
                out.push_str(" if ");
                self.write_child(out, condition, parent)?;
            }
        }
        out.extend(close);
        Some(())
    }

    /// Writes `d`'s one-row form into `out` as `{k: v, ...}`, emitting
    /// `**v` for a `None`-keyed unpacking item. A key always joins
    /// whereas a value may hold its column. `parent` is the dict itself,
    /// threaded into each child for paren recovery on non-collection
    /// leaves.
    fn write_dict(&self, out: &mut String, d: &ExprDict, parent: AnyNodeRef) -> Option<()> {
        out.push('{');
        for (i, item) in d.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            match &item.key {
                Some(key) => {
                    self.write(out, key, parent)?;
                    out.push_str(": ");
                }
                None => out.push_str("**"),
            }
            self.write_child(out, &item.value, parent)?;
        }
        out.push('}');
        Some(())
    }

    /// Writes `elts` joined by `", "` into `out`, optionally wrapped in
    /// a bracket pair and optionally followed by a trailing comma. The
    /// trailing comma carries the 1-tuple `(x,)` case.
    fn write_seq(
        &self,
        out: &mut String,
        brackets: Option<(char, char)>,
        elts: &[Expr],
        parent: AnyNodeRef,
        trailing_comma: bool,
    ) -> Option<()> {
        let (open, close) = brackets.unzip();
        out.extend(open);
        for (i, e) in elts.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.write_child(out, e, parent)?;
        }
        if trailing_comma {
            out.push(',');
        }
        out.extend(close);
        Some(())
    }
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

    #[test]
    fn form_joins_a_dict_the_cleared_explode_facet_leaves_inert() {
        let mut config = Config::default();
        config.rules.reflow_collections.explode = false;
        config.rules.reflow_collections.max_dict_entries.0 = NonZeroUsize::new(2);
        assert_eq!(
            form_under(&config, "{'a': 1,\n 'b': 2, 'c': 3}").as_deref(),
            Some("{'a': 1, 'b': 2, 'c': 3}"),
        );
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
}

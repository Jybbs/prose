//! Closes a fractured argument list back onto one row. A break the
//! author hand-wrapped closes up, whereas a flush column, a list
//! carrying a comment, and one past the argument cap each hold, so a
//! rule measuring a construct reads the width layout settles on.

use std::{borrow::Cow, cmp::Reverse};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, ArgOrKeyword, Arguments, Expr,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::CallLayoutConfig,
    primitives::{edit::apply_inline_edits, layout::is_fractured},
    source::Source,
};

/// The joins closing every fractured argument list beneath one
/// expression, ascending by start and disjoint, read back per range.
pub(crate) struct Joins(Vec<Edit>);

impl Joins {
    /// `range`'s text with every join inside it applied.
    pub(crate) fn settled<'s>(&self, source: &'s Source, range: TextRange) -> Cow<'s, str> {
        apply_inline_edits(source, range, &self.0)
    }
}

/// The terms a fracture closes under, resolved from configuration.
/// `cap` is the argument count past which a list keeps its break, and
/// `closes` is clear where `call_layout` is off and no fracture shuts
/// at all.
#[derive(Clone, Copy)]
pub(crate) struct Settings {
    cap: Option<usize>,
    closes: bool,
}

impl Settings {
    /// `arguments` joined by `", "` inside the parens, each argument
    /// settled so a nested fracture reads at its joined width. The join
    /// runs whatever `closes` holds.
    pub(crate) fn joined(self, source: &Source, arguments: &Arguments) -> String {
        join_args(source, self.cap, arguments)
    }

    /// The joins closing every fractured argument list beneath `expr`.
    pub(crate) fn joins(self, source: &Source, expr: &Expr) -> Joins {
        if !self.closes {
            return Joins(Vec::new());
        }
        Joins(join_edits(source, self.cap, expr))
    }

    /// `range`'s text with every fracture inside `expr` closed onto one
    /// line. A break that holds leaves the text spanning lines.
    pub(crate) fn text<'a>(
        self,
        source: &'a Source,
        expr: &Expr,
        range: TextRange,
    ) -> Cow<'a, str> {
        self.joins(source, expr).settled(source, range)
    }
}

impl From<&CallLayoutConfig> for Settings {
    fn from(rules: &CallLayoutConfig) -> Self {
        Self {
            cap: rules.max_args.cap(),
            closes: rules.enabled,
        }
    }
}

/// Emits one replacement per fractured argument list beneath the
/// visited expression.
struct FractureJoiner<'a> {
    cap: Option<usize>,
    edits: Vec<Edit>,
    source: &'a Source,
}

impl<'ast> AstVisitor<'ast> for FractureJoiner<'_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        walk_expr(self, expr);
        let Expr::Call(call) = expr else {
            return;
        };
        let range = call.arguments.range();
        if call.arguments.is_empty()
            || self.cap.is_some_and(|cap| call.arguments.len() > cap)
            || !is_fractured(self.source, range)
            || self.source.intersects_comment(call.arguments.inner_range())
        {
            return;
        }
        self.edits.push(Edit::range_replacement(
            join_args(self.source, self.cap, &call.arguments),
            range,
        ));
    }
}

/// `arguments` joined by `", "` inside the parens, each argument
/// settled so a nested fracture reads at its joined width.
fn join_args(source: &Source, cap: Option<usize>, arguments: &Arguments) -> String {
    format!(
        "({})",
        arguments
            .iter_source_order()
            .map(|arg| match arg {
                ArgOrKeyword::Arg(expr) => settled_argument(source, cap, expr, arguments.into()),
                ArgOrKeyword::Keyword(kw) => match &kw.arg {
                    Some(name) => Cow::Owned(format!(
                        "{name}={}",
                        settled_argument(source, cap, &kw.value, kw.into())
                    )),
                    None => Cow::Borrowed(source.slice(kw)),
                },
            })
            .join(", "),
    )
}

/// The replacement edits closing every fractured argument list beneath
/// `expr`, ascending by start and disjoint.
fn join_edits(source: &Source, cap: Option<usize>, expr: &Expr) -> Vec<Edit> {
    if !source.contains_line_break(expr.range()) {
        return Vec::new();
    }
    let mut joiner = FractureJoiner {
        cap,
        edits: Vec::new(),
        source,
    };
    joiner.visit_expr(expr);
    outermost(joiner.edits)
}

/// `edits` sorted ascending with every range an earlier edit already
/// covers dropped. A nested list is reached twice over, once on its own
/// and once inside the join its parent renders.
fn outermost(mut edits: Vec<Edit>) -> Vec<Edit> {
    edits.sort_by_key(|edit| (edit.start(), Reverse(edit.end())));
    edits.dedup_by(|edit, last| last.end() > edit.start());
    edits
}

/// One argument's text with every fractured list beneath it closed onto
/// one line. A column-shaped list keeps its break, so an enclosing
/// measure still reads it as spanning lines. An argument whose own text
/// spans rows reaches the grouping parentheses recovered against
/// `parent`, which hold those rows together once the list closes,
/// whereas a single-row argument leaves a redundant pair out of the
/// joined form.
fn settled_argument<'a>(
    source: &'a Source,
    cap: Option<usize>,
    expr: &Expr,
    parent: AnyNodeRef,
) -> Cow<'a, str> {
    let range = if source.contains_line_break(expr.range()) {
        source.paren_aware_range(expr.into(), parent)
    } else {
        expr.range()
    };
    Joins(join_edits(source, cap, expr)).settled(source, range)
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;
    use crate::{
        config::Config,
        testing::{first_expr, parse},
    };

    #[rstest]
    #[case::fracture_closes("f(g(a,\n  b), c)\n", None, "(g(a, b), c)")]
    #[case::flush_column_holds("f(g(\n    a,\n    b\n), c)\n", None, "(g(\n    a,\n    b\n), c)")]
    #[case::over_cap_holds(
        "f(g(a,\n  b,\n  c,\n  d), e)\n",
        Some(3),
        "(g(a,\n  b,\n  c,\n  d), e)"
    )]
    #[case::grouping_pair_holds("f((x.a()\n   .b()), c)\n", None, "((x.a()\n   .b()), c)")]
    #[case::single_row_grouping_pair_drops("f((a),\n  b)\n", None, "(a, b)")]
    #[case::keyword_grouping_pair_holds(
        "f(c, k=(x.a()\n   .b()))\n",
        None,
        "(c, k=(x.a()\n   .b()))"
    )]
    fn join_args_settles_each_nested_list(
        #[case] src: &str,
        #[case] cap: Option<usize>,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let call = first_expr(&source).as_call_expr().expect("a call");
        assert_eq!(join_args(&source, cap, &call.arguments), expected);
    }

    #[test]
    fn text_holds_a_list_carrying_a_comment() {
        let source = parse("f(a,  # note\n  b)\n");
        let expr = first_expr(&source);
        let settings = Config::default().fracture_settings();
        assert_matches!(settings.text(&source, expr, expr.range()), Cow::Borrowed(_));
    }

    #[test]
    fn text_holds_every_break_where_call_layout_is_off() {
        let mut config = Config::default();
        config.rules.call_layout.enabled = false;
        let source = parse("f(a,\n  b)\n");
        let expr = first_expr(&source);
        assert_matches!(
            config.fracture_settings().text(&source, expr, expr.range()),
            Cow::Borrowed(_)
        );
    }

    #[test]
    fn text_keeps_the_outermost_join_of_a_doubly_nested_fracture() {
        let source = parse("f(g(h(a,\n      b),\n  c))\n");
        let expr = first_expr(&source);
        let settings = Config::default().fracture_settings();
        assert_eq!(
            settings.text(&source, expr, expr.range()),
            "f(g(h(a, b), c))"
        );
    }
}

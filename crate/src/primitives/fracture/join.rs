//! Closes a fractured bracketed construct back onto one row.

use std::{borrow::Cow, cmp::Reverse};

use super::*;

/// Emits one replacement per fractured argument list beneath the
/// visited expression.
pub(super) struct FractureJoiner<'a> {
    pub(super) edits: Vec<Edit>,
    pub(super) settings: Settings<'a>,
    pub(super) source: &'a Source,
}

impl<'ast> AstVisitor<'ast> for FractureJoiner<'_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        walk_expr(self, expr);
        let Expr::Call(call) = expr else {
            return;
        };
        let range = call.arguments.range();
        if call.arguments.is_empty()
            || self.settings.explodes(self.source, call)
            || !is_fractured(self.source, range)
            || self.source.intersects_comment(call.arguments.inner_range())
        {
            return;
        }
        self.edits.push(Edit::range_replacement(
            join_args(self.source, self.settings, &call.arguments),
            range,
        ));
    }
}

/// `edits` sorted ascending with every range an earlier edit already
/// covers dropped. A nested list is reached twice over, once on its own
/// and once inside the join its parent renders.
pub(crate) fn outermost(mut edits: Vec<Edit>) -> Vec<Edit> {
    edits.sort_by_key(|edit| (edit.start(), Reverse(edit.end())));
    edits.dedup_by(|edit, last| last.end() > edit.start());
    edits
}

/// `arguments` joined by `", "` inside the parens, each argument
/// settled so a nested fracture reads at its joined width.
pub(super) fn join_args(source: &Source, settings: Settings<'_>, arguments: &Arguments) -> String {
    format!(
        "({})",
        arguments
            .iter_source_order()
            .map(|arg| match arg {
                ArgOrKeyword::Arg(expr) =>
                    settled_argument(source, settings, expr, arguments.into()),
                ArgOrKeyword::Keyword(kw) => match &kw.arg {
                    Some(name) => Cow::Owned(format!(
                        "{name}={}",
                        settled_argument(source, settings, &kw.value, kw.into())
                    )),
                    None => Cow::Borrowed(source.slice(kw)),
                },
            })
            .join(", "),
    )
}

/// The replacement edits closing every fractured argument list beneath
/// `expr`, ascending by start and disjoint.
pub(super) fn join_edits(source: &Source, settings: Settings<'_>, expr: &Expr) -> Vec<Edit> {
    if !source.contains_line_break(expr.range()) {
        return Vec::new();
    }
    let mut joiner = FractureJoiner {
        edits: Vec::new(),
        settings,
        source,
    };
    joiner.visit_expr(expr);
    outermost(joiner.edits)
}

/// One argument's text with every fractured list beneath it closed onto
/// one line. A column-shaped list keeps its break, so an enclosing
/// measure still reads it as spanning lines. An argument whose own text
/// spans rows reaches the grouping parentheses recovered against
/// `parent`, which hold those rows together once the list closes,
/// whereas a single-row argument leaves a redundant pair out of the
/// joined form.
pub(super) fn settled_argument<'a>(
    source: &'a Source,
    settings: Settings<'_>,
    expr: &Expr,
    parent: AnyNodeRef,
) -> Cow<'a, str> {
    let range = source.spanning_paren_range(expr.into(), parent);
    Joins(join_edits(source, settings, expr)).settled(source, range)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_expr, parse};

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
        let settings = Settings {
            cap,
            closes: true,
            targets: None,
        };
        assert_eq!(join_args(&source, settings, &call.arguments), expected);
    }
}

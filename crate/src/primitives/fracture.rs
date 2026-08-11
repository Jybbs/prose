//! Closes a fractured argument list back onto one row. A break the
//! author hand-wrapped closes up, whereas a flush column, a list
//! carrying a comment, and one past the argument cap each hold, so a
//! rule measuring a construct reads the width layout settles on.

use std::{borrow::Cow, cmp::Reverse};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, ArgOrKeyword, Arguments, Expr, ExprCall,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::CallLayoutConfig,
    primitives::{
        call_keywords::{CallTargets, takes_keyword_form},
        edit::apply_inline_edits,
        layout::is_fractured,
    },
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
pub(crate) struct Settings<'a> {
    cap: Option<usize>,
    closes: bool,
    targets: Option<&'a CallTargets<'a>>,
}

impl Settings<'_> {
    /// These settings resolving a call against `targets`, the map
    /// [`module_call_params`] builds for one source, so the join reads
    /// the count trigger the way `call-layout` explodes it.
    pub(crate) fn against<'t>(self, targets: &'t CallTargets<'t>) -> Settings<'t> {
        Settings {
            cap: self.cap,
            closes: self.closes,
            targets: Some(targets),
        }
    }

    /// True where `call-layout`'s count trigger explodes `call`, its
    /// arguments past the cap and every one taking keyword form.
    pub(crate) fn explodes(self, source: &Source, call: &ExprCall) -> bool {
        self.over_cap(call.arguments.len()) && takes_keyword_form(source, call, self.targets)
    }

    /// The joins closing every fractured argument list beneath `expr`.
    pub(crate) fn joins(self, source: &Source, expr: &Expr) -> Joins {
        if !self.closes {
            return Joins(Vec::new());
        }
        Joins(join_edits(source, self, expr))
    }

    /// True where a list of `count` arguments sits past the cap a
    /// closing fracture holds to, the list `call-layout` explodes on its
    /// count trigger. False throughout where `call-layout` is off.
    fn over_cap(self, count: usize) -> bool {
        self.closes && self.cap.is_some_and(|cap| count > cap)
    }
}

impl From<&CallLayoutConfig> for Settings<'_> {
    fn from(rules: &CallLayoutConfig) -> Self {
        Self {
            cap: rules.max_args.cap(),
            closes: rules.enabled,
            targets: None,
        }
    }
}

/// Emits one replacement per fractured argument list beneath the
/// visited expression.
struct FractureJoiner<'a> {
    edits: Vec<Edit>,
    settings: Settings<'a>,
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
fn join_args(source: &Source, settings: Settings<'_>, arguments: &Arguments) -> String {
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
fn join_edits(source: &Source, settings: Settings<'_>, expr: &Expr) -> Vec<Edit> {
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
fn settled_argument<'a>(
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

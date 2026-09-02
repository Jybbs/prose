//! Rewrites one type expression into its PEP 585 and PEP 604 forms,
//! descending through every member so a nested legacy spelling lands in
//! the same pass as the one enclosing it.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_python_ast::{
    Expr, ExprBinOp, ExprList, ExprSubscript, ExprTuple, Operator,
    name::{QualifiedName, UnqualifiedName},
};
use ruff_python_stdlib::typing::as_pep_585_generic;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use super::typing_imports::is_typing_root;
use crate::{primitives::edit::splice_bodies, source::Source};

/// Rewrites a type expression against the module's `typing` aliases,
/// recording the bound name each rewritten head read.
pub(super) struct Renderer<'a> {
    consumed: Vec<&'a str>,
    generics: bool,
    imports: &'a FxHashMap<&'a str, QualifiedName<'a>>,
    source: &'a Source,
    unions: bool,
}

impl<'a> Renderer<'a> {
    pub(super) fn new(
        source: &'a Source,
        imports: &'a FxHashMap<&'a str, QualifiedName<'a>>,
        generics: bool,
        unions: bool,
    ) -> Self {
        Self {
            consumed: Vec::new(),
            generics,
            imports,
            source,
            unions,
        }
    }

    /// The bound name `expr` reads through and the builtin PEP 585 gave
    /// the `typing` generic it names. `None` when the facet is off, when
    /// `expr` names no `typing` member, and when that member's
    /// replacement lives under `collections` rather than in builtins.
    fn builtin_for(&self, expr: &'a Expr) -> Option<(&'a str, &'static str)> {
        if !self.generics {
            return None;
        }
        let (bound, member) = self.typing_member(expr)?;
        let (module, builtin) = as_pep_585_generic("typing", member)?;
        module.is_empty().then_some((bound, builtin))
    }

    /// The builtin standing in for a `typing` generic head, the head
    /// itself for every other name or attribute.
    fn builtin_head(&mut self, expr: &'a Expr) -> Cow<'a, str> {
        match self.builtin_for(expr) {
            Some((bound, builtin)) => {
                self.consumed.push(bound);
                Cow::Owned(builtin.to_owned())
            }
            None => Cow::Borrowed(self.source.slice(expr)),
        }
    }

    /// The rewritten text of `expr`, borrowing the source slice when
    /// nothing inside it changes.
    fn rendered(&mut self, expr: &'a Expr) -> Cow<'a, str> {
        match expr {
            Expr::Attribute(_) | Expr::Name(_) => self.builtin_head(expr),
            Expr::BinOp(ExprBinOp { left, right, .. }) => {
                self.spliced(expr.range(), [left.as_ref(), right.as_ref()])
            }
            Expr::List(ExprList { elts, .. }) | Expr::Tuple(ExprTuple { elts, .. }) => {
                self.spliced(expr.range(), elts)
            }
            Expr::Starred(starred) => self.spliced(expr.range(), [starred.value.as_ref()]),
            Expr::Subscript(subscript) => self.subscript(subscript),
            _ => Cow::Borrowed(self.source.slice(expr)),
        }
    }

    /// Weaves each element's rewritten text back into `span`, keeping the
    /// brackets, separators, and spacing between them verbatim. Borrows
    /// `span` when no element changes.
    fn spliced(
        &mut self,
        span: TextRange,
        elements: impl IntoIterator<Item = &'a Expr>,
    ) -> Cow<'a, str> {
        let bodies: Vec<(Cow<'a, str>, TextRange)> = elements
            .into_iter()
            .map(|element| (self.rendered(element), element.range()))
            .collect();
        splice_bodies(self.source, span, bodies, &[])
    }

    /// The rewritten text of a subscript, preferring the PEP 604 form
    /// where the head is a legacy union and otherwise weaving the head
    /// and slice back around their brackets.
    fn subscript(&mut self, subscript: &'a ExprSubscript) -> Cow<'a, str> {
        match self.union(subscript) {
            Some(text) => Cow::Owned(text),
            None => self.spliced(
                subscript.range,
                [subscript.value.as_ref(), subscript.slice.as_ref()],
            ),
        }
    }

    /// The bound name a head reads through and the `typing` member it
    /// names, `None` when it names no `typing` member. Under
    /// `from typing import Optional as Opt`, `Opt` pairs `Opt` with
    /// `Optional`.
    fn typing_member(&self, expr: &'a Expr) -> Option<(&'a str, &'a str)> {
        let unqualified = UnqualifiedName::from_expr(expr)?;
        let (&bound, tail) = unqualified.segments().split_first()?;
        let qualified = self
            .imports
            .get(bound)?
            .clone()
            .extend_members(tail.iter().copied());
        match qualified.segments() {
            [root, member] if is_typing_root(root) => Some((bound, *member)),
            _ => None,
        }
    }

    /// The PEP 604 text of an `Optional` or `Union` subscript. `None`
    /// when the head is neither, when the facet is off, when a comment
    /// sits inside the subscript, when the subscript carries no member,
    /// and when an operand cannot carry `|`.
    fn union(&mut self, subscript: &'a ExprSubscript) -> Option<String> {
        if !self.unions {
            return None;
        }
        let (bound, member) = self.typing_member(&subscript.value)?;
        let tail = match member {
            "Optional" => " | None",
            "Union" => "",
            _ => return None,
        };
        if self.source.intersects_comment(subscript) {
            return None;
        }
        let members: &[Expr] = match subscript.slice.as_ref() {
            Expr::Tuple(tuple) => &tuple.elts,
            other => std::slice::from_ref(other),
        };
        if members.is_empty() || !members.iter().all(carries_pipe) {
            return None;
        }
        let joined = members.iter().map(|arm| self.rendered(arm)).join(" | ");
        self.consumed.push(bound);
        Some(format!("{joined}{tail}"))
    }

    /// The rewritten text of `expr` and the bound names its rewritten
    /// heads read, `None` when nothing inside `expr` changes.
    pub(super) fn rewrite(&mut self, expr: &'a Expr) -> Option<Rewrite<'a>> {
        match self.rendered(expr) {
            Cow::Borrowed(_) => None,
            Cow::Owned(text) => Some(Rewrite {
                consumed: std::mem::take(&mut self.consumed),
                text,
            }),
        }
    }
}

/// One rewritten expression, paired with the module-bound name each of
/// its rewritten heads read, once per head.
pub(super) struct Rewrite<'a> {
    pub(super) consumed: Vec<&'a str>,
    pub(super) text: String,
}

/// True when `expr` names something PEP 604's `|` accepts as an operand.
/// A forward-reference string, a PEP 646 unpack, and every shape `|`
/// would bind tighter than all read false.
fn carries_pipe(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(_) | Expr::Name(_) | Expr::NoneLiteral(_) | Expr::Subscript(_) => true,
        Expr::BinOp(ExprBinOp {
            left, op, right, ..
        }) => *op == Operator::BitOr && carries_pipe(left) && carries_pipe(right),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case("int", true)]
    #[case("pkg.Type", true)]
    #[case("None", true)]
    #[case("list[int]", true)]
    #[case("int | None", true)]
    #[case("int | list[str] | None", true)]
    #[case("\"Node\"", false)]
    #[case("1", false)]
    #[case("[int]", false)]
    #[case("int & str", false)]
    #[case("int | \"Node\"", false)]
    #[case("int if flag else str", false)]
    fn carries_pipe_admits_only_operands_a_union_may_join(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(&format!("x: {src}\n"));
        let annotation = source.ast().body[0]
            .as_ann_assign_stmt()
            .expect("an annotated assignment")
            .annotation
            .as_ref();
        assert_eq!(carries_pipe(annotation), expected, "{src}");
    }
}

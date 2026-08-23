//! Sheds the arguments of a parameterized `super(C, self)` call, leaving
//! the bare `super()`. The rewrite fires where the first argument
//! names the one enclosing class and the second names the enclosing
//! callable's first positional parameter, the pair the bare form reads
//! from the implicit `__class__` cell and the frame. A comprehension, a
//! callable taking no positional parameter, a `@dataclass(slots=True)`
//! class, a comment inside the argument list, a scope binding the class
//! name, a module binding `super` or `__class__`, and a row inside a
//! row-spanning string the deletion would leave measured against a
//! moved column each hold the arguments in place. Any other
//! continuation row the deletion would strand re-seats by the columns
//! the span took.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, Stmt, StmtClassDef,
    visitor::{Visitor, walk_expr},
};
use ruff_source_file::{LineRanges, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        binding::BindingAnalysis, decorator::is_slots_dataclass, edit::insert_edit,
        inline::indent_width, params::first_positional, reseat::push_reseat_edits,
        travel::frozen_rows, walk::walk_stmt,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ShedSuperArgs;

impl ShedSuperArgs {
    pub(crate) const MESSAGE: &'static str =
        "shed the arguments a parameterized `super()` call restates";

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for ShedSuperArgs {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let analysis = source.binding_analysis();
        if analysis.binds_name("__class__") || analysis.binds_name("super") {
            return Vec::new();
        }
        let mut walker = Walker {
            analysis,
            classes: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
            source,
            statement: TextSize::default(),
        };
        walker.visit_body(&source.ast().body);
        walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// One enclosing callable. `receiver` names the parameter the bare form
/// reads as the instance, `None` for a comprehension and for a callable
/// whose leading slot is keyword-only or variadic. `scope` carries the
/// `def` whose locals the class name resolves against, `None` for a
/// lambda and a comprehension. `class_depth` is the enclosing class
/// count where the callable opened.
struct Frame<'a> {
    class_depth: usize,
    receiver: Option<&'a str>,
    scope: Option<&'a Stmt>,
}

/// Collects one fix group per rewritable `super(...)` call, carrying the
/// enclosing class stack, the callable frame stack the walk maintains,
/// and the start of the statement under visit.
struct Walker<'a> {
    analysis: &'a BindingAnalysis,
    classes: Vec<&'a StmtClassDef>,
    frames: Vec<Frame<'a>>,
    groups: Vec<Vec<Edit>>,
    source: &'a Source,
    statement: TextSize,
}

impl<'a> Walker<'a> {
    /// Pushes a callable frame, runs `walk` inside it, and pops it.
    fn in_frame(
        &mut self,
        receiver: Option<&'a str>,
        scope: Option<&'a Stmt>,
        walk: impl FnOnce(&mut Self),
    ) {
        self.frames.push(Frame {
            class_depth: self.classes.len(),
            receiver,
            scope,
        });
        walk(self);
        self.frames.pop();
    }

    /// The edits deleting `call`'s arguments and re-seating any later
    /// row of the logical line aligned to text the deletion moves,
    /// `None` where the bare form would resolve a different class or
    /// instance, or none at all, and `None` where a stranded row sits
    /// inside a string no move re-seats. A span written across rows
    /// joins the rows it spans, so the rows below it hold.
    fn rewrite(&self, call: &ExprCall) -> Option<Vec<Edit>> {
        if call.func.as_name_expr()?.id.as_str() != "super" || !call.arguments.keywords.is_empty() {
            return None;
        }
        let [Expr::Name(class_arg), Expr::Name(instance_arg)] = &*call.arguments.args else {
            return None;
        };
        let frame = self.frames.last()?;
        let class = self.classes.last()?;
        let depth = self.classes.len();
        let name = class_arg.id.as_str();
        let span = call.arguments.inner_range();
        if frame.class_depth != depth
            || frame.receiver != Some(instance_arg.id.as_str())
            || !names_the_class(name, class, depth)
            || self.shadows(name)
            || is_slots_dataclass(class)
            || self.source.intersects_comment(span)
        {
            return None;
        }
        let tail = self.source.logical_line_tail(span.end());
        let column = self.source.column_of(span.start());
        if self.strands_a_string_row(tail, column) {
            return None;
        }
        let removal = Edit::range_deletion(span);
        let mut edits = Vec::new();
        if !self.source.contains_line_break(span) {
            let line = TextRange::new(self.source.text().line_start(span.start()), tail.end());
            push_reseat_edits(
                self.source,
                line,
                std::slice::from_ref(&removal),
                &mut edits,
            );
        }
        insert_edit(&mut edits, removal);
        Some(edits)
    }

    /// True when an enclosing `def` binds `name` in its own scope,
    /// leaving the argument reading that binding rather than the class.
    fn shadows(&self, name: &str) -> bool {
        self.frames
            .iter()
            .filter_map(|frame| frame.scope)
            .any(|scope| self.analysis.scope_binds(scope, name))
    }

    /// True when a row of `tail` inside a row-spanning string opens at
    /// `column` or past it, a row no move re-seats.
    fn strands_a_string_row(&self, tail: TextRange, column: usize) -> bool {
        self.source
            .slice(tail)
            .universal_newlines()
            .zip(frozen_rows(self.source, tail))
            .any(|(line, frozen)| frozen && indent_width(&line) >= column)
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(call) => {
                if let Some(edits) = self.rewrite(call) {
                    self.groups.push(edits);
                }
                walk_expr(self, expr);
            }
            Expr::DictComp(_) | Expr::Generator(_) | Expr::ListComp(_) | Expr::SetComp(_) => {
                self.in_frame(None, None, |walker| walk_expr(walker, expr));
            }
            Expr::Lambda(lambda) => {
                let first = lambda
                    .parameters
                    .as_deref()
                    .and_then(first_positional)
                    .map(|p| p.name().as_str());
                self.in_frame(first, None, |walker| walk_expr(walker, expr));
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.statement = stmt.start();
        match stmt {
            Stmt::ClassDef(class) => {
                self.classes.push(class);
                walk_stmt(self, stmt);
                self.classes.pop();
            }
            Stmt::FunctionDef(function) => {
                let first = first_positional(&function.parameters).map(|p| p.name().as_str());
                self.in_frame(first, Some(stmt), |walker| walk_stmt(walker, stmt));
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

/// True when `name` reaches `class` from a method body: the implicit
/// `__class__` cell at any nesting, or the class's own name where
/// `depth` counts one enclosing class, since a nested class binds its
/// name outside the method's reach.
fn names_the_class(name: &str, class: &StmtClassDef, depth: usize) -> bool {
    name == "__class__" || (depth == 1 && name == class.name.as_str())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::no_enclosing_callable("class C:\n    marker = super(C, marker)\n")]
    #[case::class_nested_in_a_callable(
        "def f(self):\n    class C:\n        marker = super(C, self)\n"
    )]
    fn degenerate_scope_holds_the_call(#[case] src: &str) {
        assert!(ShedSuperArgs.apply(&parse(src)).is_empty());
    }

    #[test]
    fn deletes_only_the_span_between_the_parentheses() {
        let source = parse("class C:\n    def m(self):\n        return super(C, self).m()\n");
        let groups = ShedSuperArgs.apply(&source);
        let [edit] = groups[0].as_slice() else {
            panic!("a call on one row deletes its span alone");
        };
        assert!(edit.is_deletion());
        assert_eq!(&source.text()[edit.range()], "C, self");
    }

    #[rstest]
    fn reserved_name_binding_holds_every_call(#[values("__class__", "super")] name: &str) {
        let source = parse(&format!(
            "class C:\n    def m(self):\n        {name} = 1\n        return super(C, self).m()\n"
        ));
        assert!(ShedSuperArgs.apply(&source).is_empty());
    }

    #[rstest]
    #[case::aligned_continuation(
        "class C:\n    def m(self, a, b):\n        return super(C, self).m(a,\n                                b)\n",
        Some(7)
    )]
    #[case::hanging_continuation(
        "class C:\n    def m(self, a, b):\n        return super(C, self).m(\n            a, b)\n",
        None
    )]
    #[case::enclosing_closer(
        "class C:\n    def m(self, a, b):\n        return zz(\n            super(C, self).m(a,\n                             b)\n        )\n",
        Some(7)
    )]
    #[case::spread_span(
        "class C:\n    def m(self, a, b):\n        return super(\n            C,\n            self\n        ).m(a,\n            b)\n",
        None
    )]
    fn a_continuation_re_seats_by_the_columns_the_span_took(
        #[case] src: &str,
        #[case] re_seated: Option<u32>,
    ) {
        let source = parse(src);
        let groups = ShedSuperArgs.apply(&source);
        let indents: Vec<u32> = groups[0]
            .iter()
            .filter(|edit| source.slice(edit.range()).trim().is_empty())
            .map(|edit| edit.range().len().to_u32())
            .collect();
        assert_eq!(indents.first().copied(), re_seated);
        assert_eq!(indents.len(), usize::from(re_seated.is_some()));
    }
}

//! Rewrites a parameterized `super(C, self)` call to the bare `super()`
//! by deleting its arguments. The rewrite fires where the first argument
//! names the one enclosing class and the second names the enclosing
//! callable's first positional parameter, the pair the bare form reads
//! from the implicit `__class__` cell and the frame. A comprehension, a
//! callable taking no positional parameter, a `@dataclass(slots=True)`
//! class, a comment inside the argument list, a scope binding the class
//! name, and a module binding `super` or `__class__` each hold the
//! arguments in place.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, Stmt, StmtClassDef,
    visitor::{Visitor, walk_expr, walk_stmt},
};

use crate::{
    config::Config,
    primitives::{
        binding::BindingAnalysis, decorator::is_slots_dataclass, edit::singleton_groups,
        params::first_positional,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct BareSuper;

impl BareSuper {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for BareSuper {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let analysis = source.binding_analysis();
        if analysis.binds_anywhere("__class__") || analysis.binds_anywhere("super") {
            return Vec::new();
        }
        let mut walker = Walker {
            analysis,
            classes: Vec::new(),
            edits: Vec::new(),
            frames: Vec::new(),
            source,
        };
        walker.visit_body(&source.ast().body);
        singleton_groups(walker.edits)
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

/// Collects one deletion per rewritable `super(...)` call, carrying the
/// enclosing class stack and the callable frame stack the walk maintains.
struct Walker<'a> {
    analysis: &'a BindingAnalysis,
    classes: Vec<&'a StmtClassDef>,
    edits: Vec<Edit>,
    frames: Vec<Frame<'a>>,
    source: &'a Source,
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

    /// The edit deleting `call`'s arguments, `None` where the bare form
    /// would resolve a different class or instance, or none at all.
    fn rewrite(&self, call: &ExprCall) -> Option<Edit> {
        if call.func.as_name_expr()?.id.as_str() != "super" || !call.arguments.keywords.is_empty() {
            return None;
        }
        let [Expr::Name(class_arg), Expr::Name(instance_arg)] = &*call.arguments.args else {
            return None;
        };
        let frame = self.frames.last()?;
        let class = self.classes.last()?;
        let span = call.arguments.inner_range();
        if frame.class_depth != self.classes.len()
            || frame.receiver != Some(instance_arg.id.as_str())
            || !names_the_class(class_arg.id.as_str(), class, self.classes.len())
            || self.shadows(class_arg.id.as_str())
            || is_slots_dataclass(class)
            || self.source.intersects_comment(span)
        {
            return None;
        }
        Some(Edit::range_deletion(span))
    }

    /// True when an enclosing `def` binds `name` in its own scope,
    /// leaving the argument reading that binding rather than the class.
    fn shadows(&self, name: &str) -> bool {
        self.frames
            .iter()
            .filter_map(|frame| frame.scope)
            .any(|scope| self.analysis.scope_binds(scope, name))
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(call) => {
                if let Some(edit) = self.rewrite(call) {
                    self.edits.push(edit);
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
    #[case("class C:\n    marker = super(C, marker)\n")]
    #[case("def f(self):\n    class C:\n        marker = super(C, self)\n")]
    fn degenerate_scope_holds_the_call(#[case] src: &str) {
        assert!(BareSuper.apply(&parse(src)).is_empty());
    }

    #[test]
    fn deletes_only_the_span_between_the_parentheses() {
        let source = parse("class C:\n    def m(self):\n        return super(C, self).m()\n");
        let groups = BareSuper.apply(&source);
        let edit = &groups[0][0];
        assert!(edit.is_deletion());
        assert_eq!(&source.text()[edit.range()], "C, self");
    }

    #[rstest]
    fn reserved_name_binding_holds_every_call(#[values("__class__", "super")] name: &str) {
        let source = parse(&format!(
            "class C:\n    def m(self):\n        {name} = 1\n        return super(C, self).m()\n"
        ));
        assert!(BareSuper.apply(&source).is_empty());
    }
}

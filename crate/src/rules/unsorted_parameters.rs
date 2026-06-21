//! Flags a free function's positional-or-keyword parameters when they
//! sit out of alphabetical order. Report-only, since reordering them
//! rebinds every positional call site and a single-file formatter
//! cannot prove that every caller binds by keyword.
//!
//! A class-body `def` (a method, callers outside the module) and a
//! positional-binding-decorated function are skipped whole. `self` /
//! `cls` and positional-only parameters merely drop from the check,
//! leaving the rest of the signature still evaluated.

use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        params::{params_unsorted, pins_positional_params},
        scope::{BodyScope, scoped_body},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct UnsortedParameters;

impl UnsortedParameters {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for UnsortedParameters {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut visitor = Visitor {
            diagnostics: Vec::new(),
            message: self.message(),
            rule: self.id(),
            scope: BodyScope::Module,
        };
        visitor.visit_body(&source.ast().body);
        visitor.diagnostics
    }
}

struct Visitor {
    diagnostics: Vec<Diagnostic>,
    message: &'static str,
    rule: RuleId,
    scope: BodyScope,
}

impl<'a> StatementVisitor<'a> for Visitor {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(f) = stmt
            && self.scope != BodyScope::Class
            && !pins_positional_params(f)
            && params_unsorted(&f.parameters)
        {
            self.diagnostics.push(Diagnostic::lint(
                self.rule,
                f.parameters.range(),
                self.message.to_owned(),
            ));
        }
        let enclosing = self.scope;
        self.scope = scoped_body(stmt).map_or(enclosing, |(_, s)| s);
        walk_stmt(self, stmt);
        self.scope = enclosing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diagnostics::Severity, testing::parse};

    #[test]
    fn flags_an_unsorted_free_function() {
        let source = parse("def merge(target, source): pass\n");
        let diagnostics = UnsortedParameters.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");
        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert_eq!(&source.text()[only.range], "(target, source)");
    }

    #[test]
    fn leaves_a_sorted_free_function_quiet() {
        let source = parse("def merge(source, target): pass\n");
        assert!(UnsortedParameters.lint(&source).is_empty());
    }

    #[test]
    fn stays_silent_on_a_method() {
        let source = parse("class C:\n    def m(self, target, source): pass\n");
        assert!(UnsortedParameters.lint(&source).is_empty());
    }

    #[test]
    fn stays_silent_on_a_positional_binding_decorator() {
        let source = parse("@click.argument(\"x\")\ndef f(target, source): pass\n");
        assert!(UnsortedParameters.lint(&source).is_empty());
    }

    #[test]
    fn flags_a_function_nested_in_a_method() {
        let source = parse("class C:\n    def m(self):\n        def inner(target, source): pass\n");
        let diagnostics = UnsortedParameters.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");
        assert_eq!(&source.text()[only.range], "(target, source)");
    }

    #[test]
    fn flags_a_free_function_after_a_class() {
        let source = parse(
            "class C:\n    def m(self, target, source): pass\n\n\ndef free(target, source): pass\n",
        );
        let diagnostics = UnsortedParameters.lint(&source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(&source.text()[diagnostics[0].range], "(target, source)");
    }

    #[test]
    fn flags_a_free_function_under_a_module_compound_arm() {
        let source = parse("if True:\n    def f(target, source): pass\n");
        let diagnostics = UnsortedParameters.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");
        assert_eq!(&source.text()[only.range], "(target, source)");
    }

    #[test]
    fn stays_silent_on_a_method_under_a_compound_arm() {
        let source = parse("class C:\n    if True:\n        def m(self, target, source): pass\n");
        assert!(UnsortedParameters.lint(&source).is_empty());
    }

    #[test]
    fn respects_the_required_before_optional_partition() {
        let source = parse("def f(source, target, fallback=None): pass\n");
        assert!(UnsortedParameters.lint(&source).is_empty());
    }
}

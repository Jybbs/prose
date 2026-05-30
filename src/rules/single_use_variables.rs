//! Flags function-local bindings that are written exactly once and
//! read exactly once, where inlining the right-hand side into the use
//! site is usually more direct. Lint-only, emits no edits.
//!
//! Conservative skips absorb the false-positive surfaces:
//!
//! - Functions whose body declares `global` or `nonlocal` are skipped
//!   entirely, since the scope analysis becomes cross-function.
//! - Comprehension targets are skipped, since their bindings live in
//!   the comprehension's own scope rather than the enclosing function.
//! - Augmented assignments are skipped, since `x += 1` is both a read
//!   and a write of `x`.
//! - Names matching the configurable `allow_pattern` regex (default
//!   `^_`) are skipped, exempting `_unused` and similar.
//! - Only `Assignment` and `Walrus` writes flag, leaving parameters,
//!   loop targets, `with`-targets, exception handlers, and nested
//!   `def`/`class` bindings out of the diagnostic surface.

use regex_lite::Regex;
use ruff_python_ast::{
    Expr, Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{BindingAnalysis, BindingId, BindingKind},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct SingleUseVariables {
    allow_pattern: Regex,
}

impl SingleUseVariables {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow_pattern: config.rules.single_use_variables.allow_pattern.clone(),
        }
    }
}

impl Rule for SingleUseVariables {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut visitor = Visitor {
            allow_pattern: &self.allow_pattern,
            analysis: source.binding_analysis(),
            diagnostics: Vec::new(),
            rule: self.id(),
            text: source.text(),
        };
        visitor.visit_body(&source.ast().body);
        visitor.diagnostics
    }
}

struct ScopeModifierWalker {
    found: bool,
}

impl<'a> StatementVisitor<'a> for ScopeModifierWalker {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.found {
            return;
        }
        match stmt {
            Stmt::Global(_) | Stmt::Nonlocal(_) => self.found = true,
            _ => walk_stmt(self, stmt),
        }
    }
}

struct Visitor<'a> {
    allow_pattern: &'a Regex,
    analysis: &'a BindingAnalysis,
    diagnostics: Vec<Diagnostic>,
    rule: RuleId,
    text: &'a str,
}

impl Visitor<'_> {
    fn candidate(&self, binding: BindingId, body: &[Stmt]) -> Option<Diagnostic> {
        if !matches!(
            self.analysis.binding_kinds(binding),
            [BindingKind::Assignment | BindingKind::Walrus],
        ) {
            return None;
        }
        if self.analysis.assignment_count(binding) != 1 || self.analysis.usage_count(binding) != 1 {
            return None;
        }
        let name = self.analysis.binding_name(binding);
        if self.allow_pattern.is_match(name) {
            return None;
        }
        let write_offset = self.analysis.first_write_offset(binding);
        let message = match assignment_value_range(body, write_offset) {
            Some(range) => format!(
                "`{name}` is assigned and used once. Consider inlining `{}`",
                &self.text[range],
            ),
            None => format!("`{name}` is assigned and used once. Consider inlining"),
        };
        Some(Diagnostic::lint(
            self.rule,
            TextRange::at(write_offset, TextSize::of(name)),
            message,
        ))
    }

    fn flag_function_locals(&mut self, body: &[Stmt], stmt: &Stmt) {
        if body_uses_scope_modifier(body) {
            return;
        }
        for binding in self.analysis.bindings_in_scope(stmt) {
            if let Some(diagnostic) = self.candidate(binding, body) {
                self.diagnostics.push(diagnostic);
            }
        }
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(function) = stmt {
            self.flag_function_locals(&function.body, stmt);
        }
        walk_stmt(self, stmt);
    }
}

/// Returns `true` when `body` declares `global` or `nonlocal` anywhere
/// in its lexical tree, including inside nested `def` or `class`
/// scopes. A nested `nonlocal` reaches back into this function and
/// inflates the enclosing scope's usage counts, so the rule treats
/// any descendant scope modifier as a signal that the analysis is
/// no longer reliable.
fn body_uses_scope_modifier(body: &[Stmt]) -> bool {
    let mut walker = ScopeModifierWalker { found: false };
    walker.visit_body(body);
    walker.found
}

/// Returns the source range of the value bound to the name written at
/// `target_offset`, so the inlining suggestion can name what would be
/// substituted. Descends through compound statements but stops at
/// nested `def` and `class` scopes, where a same-named binding belongs
/// to another scope.
fn assignment_value_range(body: &[Stmt], target_offset: TextSize) -> Option<TextRange> {
    let mut finder = AssignmentValueFinder {
        target_offset,
        value_range: None,
    };
    finder.visit_body(body);
    finder.value_range
}

fn name_at_offset(expr: &Expr, offset: TextSize) -> bool {
    matches!(expr, Expr::Name(name) if name.range().start() == offset)
}

struct AssignmentValueFinder {
    target_offset: TextSize,
    value_range: Option<TextRange>,
}

impl<'a> StatementVisitor<'a> for AssignmentValueFinder {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.value_range.is_some() {
            return;
        }
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Assign(assign)
                if assign
                    .targets
                    .iter()
                    .any(|target| name_at_offset(target, self.target_offset)) =>
            {
                self.value_range = Some(assign.value.range());
            }
            Stmt::AnnAssign(annotation)
                if annotation.value.is_some()
                    && name_at_offset(&annotation.target, self.target_offset) =>
            {
                self.value_range = annotation.value.as_ref().map(|value| value.range());
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::test_support::parse;

    fn first_function_body(source: &Source) -> &[Stmt] {
        &source.ast().body[0]
            .as_function_def_stmt()
            .expect("function def")
            .body
    }

    #[test]
    fn body_uses_scope_modifier_descends_into_nested_function() {
        let source = parse("def outer():\n    def inner():\n        nonlocal x\n");
        assert!(body_uses_scope_modifier(first_function_body(&source)));
    }

    #[test]
    fn body_uses_scope_modifier_finds_global_in_nested_block() {
        let source = parse("def f():\n    if cond:\n        global x\n");
        assert!(body_uses_scope_modifier(first_function_body(&source)));
    }

    #[test]
    fn body_uses_scope_modifier_returns_false_on_clean_body() {
        let source = parse("def f():\n    x = 1\n    return x\n");
        assert!(!body_uses_scope_modifier(first_function_body(&source)));
    }

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_range_over_name() {
        let source = parse("def f():\n    x = 1\n    return x\n");
        let rule = SingleUseVariables::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.contains("`x`"));
        assert!(only.message.ends_with("Consider inlining `1`"));
        assert_eq!(&source.text()[only.range], "x");
    }

    #[test]
    fn message_carries_inlined_value_from_nested_block() {
        let source = parse("def f():\n    if cond:\n        y = g() + 1\n        return y\n");
        let rule = SingleUseVariables::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert!(only.message.ends_with("Consider inlining `g() + 1`"));
    }
}

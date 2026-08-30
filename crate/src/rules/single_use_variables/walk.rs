//! Walks each scope collecting the bindings read exactly once.

use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{TextRange, TextSize};

use super::*;

pub(super) struct Visitor<'a> {
    pub(super) allow_pattern: &'a AllowPattern,
    pub(super) analysis: &'a BindingAnalysis,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) rule: RuleId,
    pub(super) text: &'a str,
}

impl Visitor<'_> {
    fn candidate(&self, binding: BindingId) -> Option<Diagnostic> {
        if !matches!(
            self.analysis.binding_kinds(binding),
            [BindingKind::Assignment | BindingKind::Walrus],
        ) {
            return None;
        }
        if self.analysis.assignment_count(binding) != 1 || self.analysis.usage_count(binding) != 1 {
            return None;
        }
        if self.analysis.walrus_in_condition(binding) {
            return None;
        }
        let name = self.analysis.binding_name(binding);
        if self.allow_pattern.matches(name) {
            return None;
        }
        let write_offset = self.analysis.first_write_offset(binding);
        let value = match self.analysis.unpack_target(binding) {
            Some(UnpackKind::Exempt) => return None,
            Some(UnpackKind::Suggested(range, index)) => {
                format!(" `{}[{index}]`", &self.text[range])
            }
            Some(UnpackKind::Bare) => String::new(),
            None => self
                .analysis
                .assignment_value_range(write_offset)
                .map(|range| format!(" `{}`", &self.text[range]))
                .unwrap_or_default(),
        };
        Some(Diagnostic::lint(
            self.rule,
            TextRange::at(write_offset, TextSize::of(name)),
            format!("`{name}` is assigned and used once. Consider inlining{value}"),
        ))
    }

    fn flag_function_locals(&mut self, body: &[Stmt], stmt: &Stmt) {
        if body_uses_scope_modifier(body) {
            return;
        }
        for binding in self.analysis.bindings_in_scope(stmt) {
            if let Some(diagnostic) = self.candidate(binding) {
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
pub(super) fn body_uses_scope_modifier(body: &[Stmt]) -> bool {
    any_over_stmts(body, |stmt| {
        matches!(stmt, Stmt::Global(_) | Stmt::Nonlocal(_))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{first_def, parse};

    fn first_function_body(source: &Source) -> &[Stmt] {
        &first_def(source).body
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
}

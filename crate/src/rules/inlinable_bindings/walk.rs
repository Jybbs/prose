//! Walks each function scope collecting the bindings whose inline costs
//! nothing.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_source_file::LineRanges;
use ruff_text_size::{TextRange, TextSize};

use super::{guards::guarded_regions, *};
use crate::primitives::{edit::apply_inline_edits, inline::display_width, walk::any_over_stmts};

pub(super) struct Visitor<'a> {
    pub(super) allow_pattern: &'a AllowPattern,
    pub(super) analysis: &'a BindingAnalysis,
    pub(super) code_line_length: usize,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) rule: RuleId,
    pub(super) source: &'a Source,
}

impl Visitor<'_> {
    fn candidate(&self, binding: BindingId, guards: &[TextRange]) -> Option<Diagnostic> {
        if !matches!(
            self.analysis.binding_kinds(binding),
            [BindingKind::Assignment | BindingKind::Walrus],
        ) {
            return None;
        }
        if self.analysis.assignment_count(binding) != 1 {
            return None;
        }
        let &[read] = self.analysis.read_offsets(binding) else {
            return None;
        };
        let name = self.analysis.binding_name(binding);
        if self.allow_pattern.matches(name) {
            return None;
        }
        let write = self.analysis.first_write_offset(binding);
        if guards
            .iter()
            .any(|guard| guard.contains(read) && !guard.contains(write))
        {
            return None;
        }
        let value = self.replacement(binding, write)?;
        if self.overflows(read, name, &value) {
            return None;
        }
        Some(Diagnostic::lint(
            self.rule,
            TextRange::at(write, TextSize::of(name)),
            format!("`{name}` is assigned and used once. Consider inlining `{value}`"),
        ))
    }

    fn flag_function_locals(&mut self, body: &[Stmt], stmt: &Stmt) {
        if body_uses_scope_modifier(body) {
            return;
        }
        let guards = guarded_regions(body);
        for binding in self.analysis.bindings_in_scope(stmt) {
            if let Some(diagnostic) = self.candidate(binding, &guards) {
                self.diagnostics.push(diagnostic);
            }
        }
    }

    /// Returns `true` when standing `value` in for `name` carries the
    /// read's own physical row past the code budget.
    fn overflows(&self, read: TextSize, name: &str, value: &str) -> bool {
        let swap =
            Edit::range_replacement(value.to_owned(), TextRange::at(read, TextSize::of(name)));
        let row = self.source.text().line_range(read);

        display_width(&apply_inline_edits(self.source, row, &[swap])) > self.code_line_length
    }

    /// Returns the text that would stand in the binding's place at its
    /// read, and `None` where no rewrite resolves or the value it names
    /// spans rows.
    fn replacement(&self, binding: BindingId, write: TextSize) -> Option<String> {
        let (value, index) = match self.analysis.unpack_target(binding) {
            Some(UnpackKind::Bare | UnpackKind::Exempt) => return None,
            Some(UnpackKind::Suggested(range, index)) => (range, Some(index)),
            None => (self.analysis.assignment_value_range(write)?, None),
        };
        if self.source.contains_line_break(value) {
            return None;
        }
        let text = self.source.slice(value);
        Some(match index {
            Some(index) => format!("{text}[{index}]"),
            None => text.to_owned(),
        })
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
/// scopes.
fn body_uses_scope_modifier(body: &[Stmt]) -> bool {
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

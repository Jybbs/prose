//! Flags a module-level `SCREAMING_CASE` binding that is reassigned.
//! A write-once name passes whatever its value. The structural-home
//! carve-outs (dunder names, `TypeVar` / `ParamSpec` / `NewType` /
//! `TypeAliasType` constructors, the `if TYPE_CHECKING:` block, and the
//! per-project `allow` list) drop out ahead of the reassignment gate.

use std::collections::HashSet;

use ruff_python_ast::{
    Expr, Stmt, StmtIf,
    name::UnqualifiedName,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{BindingAnalysis, annotated_name_target, single_name_target},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ReassignedConstants {
    allow: HashSet<String>,
}

impl ReassignedConstants {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: config
                .rules
                .reassigned_constants
                .allow
                .iter()
                .cloned()
                .collect(),
        }
    }
}

impl Rule for ReassignedConstants {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut walker = Walker {
            allow: &self.allow,
            analysis: source.binding_analysis(),
            diagnostics: Vec::new(),
            rule: self.id(),
        };
        walker.visit_body(&source.ast().body);
        walker.diagnostics
    }
}

struct Walker<'a> {
    allow: &'a HashSet<String>,
    analysis: &'a BindingAnalysis,
    diagnostics: Vec<Diagnostic>,
    rule: RuleId,
}

impl Walker<'_> {
    fn emit(&mut self, stmt: &Stmt, name: &str) {
        self.diagnostics.push(Diagnostic::lint(
            self.rule,
            stmt.range(),
            format!(
                "Module-level `{name}` is SCREAMING_CASE but reassigned. \
                 Rename it to lowercase or keep it write-once",
            ),
        ));
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => return,
            Stmt::If(if_stmt) if is_type_checking_block(if_stmt) => return,
            Stmt::Assign(a) => {
                if let Some(name) = single_name_target(a)
                    && is_reassigned_constant_target(name, Some(a.value.as_ref()), self.allow)
                    && self.analysis.module_reassigned(name)
                {
                    self.emit(stmt, name);
                }
            }
            Stmt::AnnAssign(a) => {
                if let Some(name) = annotated_name_target(a)
                    && is_reassigned_constant_target(name, a.value.as_deref(), self.allow)
                    && self.analysis.module_reassigned(name)
                {
                    self.emit(stmt, name);
                }
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns `true` when `name` matches `SCREAMING_CASE`, is not in the
/// per-project allowlist, and (when present) the right-hand side is not
/// a `TypeVar` / `ParamSpec` / `NewType` / `TypeAliasType` constructor.
/// `value = None` covers the bare annotation form `X: int`.
/// `SCREAMING_CASE` already rejects dunder names, which lead with `_`.
fn is_reassigned_constant_target(
    name: &str,
    value: Option<&Expr>,
    allow: &HashSet<String>,
) -> bool {
    is_screaming_case(name)
        && !allow.contains(name)
        && !value.is_some_and(is_typing_constructor_call)
}

/// Returns `true` when `id` begins with an ASCII uppercase letter and
/// every remaining character is an ASCII uppercase letter, digit, or
/// underscore.
fn is_screaming_case(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Returns `true` when `stmt.test` matches the bare `TYPE_CHECKING`
/// name or any `<...>.TYPE_CHECKING` attribute access.
fn is_type_checking_block(stmt: &StmtIf) -> bool {
    match stmt.test.as_ref() {
        Expr::Name(name) => name.id == "TYPE_CHECKING",
        Expr::Attribute(attr) => attr.attr.as_str() == "TYPE_CHECKING",
        _ => false,
    }
}

/// Returns `true` when `value` is a call whose callable resolves to
/// `TypeVar`, `ParamSpec`, `NewType`, or `TypeAliasType`, either bare
/// (`TypeVar(...)`) or attribute-qualified (`typing.TypeVar(...)`).
fn is_typing_constructor_call(value: &Expr) -> bool {
    let last = value
        .as_call_expr()
        .and_then(|call| UnqualifiedName::from_expr(&call.func))
        .and_then(|q| q.segments().last().copied());
    matches!(
        last,
        Some("NewType" | "ParamSpec" | "TypeAliasType" | "TypeVar"),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn is_screaming_case_accepts_canonical_constants(
        #[values("PI", "MAX_RETRIES", "X1", "LOG_LEVEL_2")] id: &str,
    ) {
        assert!(is_screaming_case(id));
    }

    #[rstest]
    fn is_screaming_case_rejects_mixed_and_lowercase_names(
        #[values("", "pi", "Pi", "pI", "_HIDDEN", "1ABC", "MAX_retries")] id: &str,
    ) {
        assert!(!is_screaming_case(id));
    }
}

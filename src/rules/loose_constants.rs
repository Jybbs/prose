//! Flags module-level `SCREAMING_CASE` assignments outside the
//! recognized structural-home carve-outs (dunder names, `TypeVar` /
//! `ParamSpec` / `NewType` / `TypeAliasType` constructors, the
//! `if TYPE_CHECKING:` block, and the per-project `allow` list).

use std::collections::HashSet;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
use ruff_python_ast::{Expr, ExprName, Stmt, StmtIf};
use ruff_text_size::Ranged;

use crate::config::Config;
use crate::diagnostics::Diagnostic;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct LooseConstants {
    allow: HashSet<String>,
}

impl LooseConstants {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: config.rules.loose_constants.allow.iter().cloned().collect(),
        }
    }
}

impl Rule for LooseConstants {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut walker = Walker {
            allow: &self.allow,
            diagnostics: Vec::new(),
            rule: self.id(),
        };
        walker.visit_body(&source.ast().body);
        walker.diagnostics
    }
}

struct Walker<'a> {
    allow: &'a HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    rule: RuleId,
}

impl Walker<'_> {
    fn emit(&mut self, stmt: &Stmt, name: &str) {
        self.diagnostics.push(Diagnostic::lint(
            self.rule,
            stmt.range(),
            format!(
                "module-level constant `{name}` found. \
                 Consider an enum member, a class field, or a function-local",
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
                if let [Expr::Name(target)] = a.targets.as_slice()
                    && is_loose_constant_target(target, Some(a.value.as_ref()), self.allow)
                {
                    self.emit(stmt, &target.id);
                }
            }
            Stmt::AnnAssign(a) => {
                if let Expr::Name(target) = a.target.as_ref()
                    && is_loose_constant_target(target, a.value.as_deref(), self.allow)
                {
                    self.emit(stmt, &target.id);
                }
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns `true` when `target.id` matches `SCREAMING_CASE`, is not a
/// dunder, is not in the per-project allowlist, and (when present) the
/// right-hand side is not a `TypeVar` / `ParamSpec` / `NewType` /
/// `TypeAliasType` constructor. `value = None` covers the bare
/// annotation form `X: int`.
fn is_loose_constant_target(
    target: &ExprName,
    value: Option<&Expr>,
    allow: &HashSet<String>,
) -> bool {
    is_screaming_case(&target.id)
        && !is_dunder(&target.id)
        && !allow.contains(target.id.as_str())
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

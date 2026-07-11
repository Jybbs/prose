//! Flags a module-level `SCREAMING_CASE` binding that is reassigned.
//! A write-once name passes whatever its value. The structural-home
//! carve-outs (dunder names, `TypeVar` / `ParamSpec` / `NewType` /
//! `TypeAliasType` constructors, the `if TYPE_CHECKING:` block, and the
//! per-project `allow` list) drop out ahead of the reassignment gate.

use std::collections::HashSet;

use ruff_python_ast::{
    Expr, Stmt,
    name::UnqualifiedName,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{
        BindingAnalysis, annotated_name_target, is_screaming_case, single_name_target,
        skips_module_scan,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ReassignedConstants {
    allow: HashSet<String>,
}

impl ReassignedConstants {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: Config::allow_set(&config.rules.reassigned_constants.allow),
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
        if skips_module_scan(stmt) {
            return;
        }
        match stmt {
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

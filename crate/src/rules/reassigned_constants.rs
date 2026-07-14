//! Flags a module-level `SCREAMING_CASE` binding that is reassigned.
//! A write-once name passes whatever its value. The structural-home
//! carve-outs (dunder names, `TypeVar` / `ParamSpec` / `NewType` /
//! `TypeAliasType` constructors, the `if TYPE_CHECKING:` block, and the
//! per-project `allow` list) drop out ahead of the reassignment gate.

use std::collections::HashSet;

use ruff_python_ast::{Expr, Stmt, name::UnqualifiedName};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{is_screaming_case, module_assignments},
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

    fn reassigned(&self, stmt: &Stmt, name: &str) -> Diagnostic {
        Diagnostic::lint(
            self.id(),
            stmt.range(),
            format!(
                "Module-level `{name}` is SCREAMING_CASE but reassigned. \
                 Rename it to lowercase or keep it write-once",
            ),
        )
    }
}

impl Rule for ReassignedConstants {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let analysis = source.binding_analysis();
        module_assignments(&source.ast().body)
            .iter()
            .filter(|site| {
                let name = site.target.id.as_str();
                is_reassigned_constant_target(name, site.value, &self.allow)
                    && analysis.module_reassigned(name)
            })
            .map(|site| self.reassigned(site.stmt, site.target.id.as_str()))
            .collect()
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

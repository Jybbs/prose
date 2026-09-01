//! Flags a module-level `SCREAMING_CASE` binding that is reassigned.
//! A write-once name passes whatever its value. The structural-home
//! carve-outs (dunder names, `TypeVar` / `ParamSpec` / `NewType` /
//! `TypeAliasType` constructors, the `if TYPE_CHECKING:` block, and the
//! per-project `allow` list) drop out ahead of the reassignment gate.

use ruff_python_ast::Expr;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{
        BindingAnalysis, ModuleAssignment, is_screaming_case, module_assignments, tail_identifier,
    },
    rule::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct ReassignedConstants {
    allow: FxHashSet<String>,
}

impl ReassignedConstants {
    pub(crate) const MESSAGE: &'static str = "SCREAMING_CASE name is reassigned despite its constant casing. Rename it lowercase or keep it write-once";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: Config::allow_set(&config.rules.reassigned_constants.allow),
        }
    }

    /// True when `site` binds a `SCREAMING_CASE` name outside the
    /// per-project allowlist that the module reassigns, its value (when
    /// present) being no `TypeVar` / `ParamSpec` / `NewType` /
    /// `TypeAliasType` constructor. A `None` value covers the bare
    /// annotation form `X: int`, and `SCREAMING_CASE` already rejects the
    /// dunder names, which lead with `_`.
    fn is_reassigned_constant(&self, site: &ModuleAssignment, analysis: &BindingAnalysis) -> bool {
        let name = site.target.id.as_str();
        is_screaming_case(name)
            && !self.allow.contains(name)
            && !site.value.is_some_and(is_typing_constructor_call)
            && analysis.module_reassigned(name)
    }

    fn reassigned(&self, site: &ModuleAssignment) -> Diagnostic {
        let name = site.target.id.as_str();
        Diagnostic::lint(
            self.id(),
            site.stmt.range(),
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
            .filter(|site| self.is_reassigned_constant(site, analysis))
            .map(|site| self.reassigned(site))
            .collect()
    }
}

/// Returns `true` when `value` is a call whose callable resolves to
/// `TypeVar`, `ParamSpec`, `NewType`, or `TypeAliasType`, either bare
/// (`TypeVar(...)`) or attribute-qualified (`typing.TypeVar(...)`).
fn is_typing_constructor_call(value: &Expr) -> bool {
    matches!(
        value
            .as_call_expr()
            .and_then(|call| tail_identifier(&call.func)),
        Some("NewType" | "ParamSpec" | "TypeAliasType" | "TypeVar"),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    #[rstest]
    #[case("TypeVar(\"T\")", true)]
    #[case("typing.TypeVar(\"T\")", true)]
    #[case("NewType(\"UserId\", int)", true)]
    #[case("ParamSpec(\"P\")", true)]
    #[case("TypeAliasType(\"Seconds\", float)", true)]
    #[case("registry[0].TypeVar(\"T\")", true)]
    #[case("TypeVar", false)]
    #[case("dict(timeout=30)", false)]
    #[case("42", false)]
    fn is_typing_constructor_call_reads_the_callable_tail(
        #[case] value_src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(&format!("X = {value_src}\n"));
        assert_eq!(
            is_typing_constructor_call(first_value(&source)),
            expected,
            "{value_src}"
        );
    }
}

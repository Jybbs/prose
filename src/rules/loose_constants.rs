//! Flags module-level `SCREAMING_CASE` assignments outside the
//! recognized structural-home carve-outs (dunder names, `TypeVar` /
//! `ParamSpec` / `NewType` / `TypeAliasType` constructors, the
//! `if TYPE_CHECKING:` block, and the per-project `allow` list).

use std::collections::HashSet;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::{Expr, ExprName, Stmt, StmtIf};
use ruff_text_size::Ranged;

use crate::config::Config;
use crate::diagnostics::{Diagnostic, Severity};
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
    fn apply(&self, _source: &Source) -> Vec<Edit> {
        Vec::new()
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(LooseConstants))
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
        self.diagnostics.push(Diagnostic {
            fix: None,
            message: format!(
                "module-level constant `{name}` found. \
                 Consider an enum member, a class field, or a function-local",
            ),
            range: stmt.range(),
            rule: self.rule,
            severity: Severity::Lint,
        });
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => return,
            Stmt::If(if_stmt) if is_type_checking_block(if_stmt) => return,
            Stmt::Assign(a) => {
                if let [Expr::Name(target)] = a.targets.as_slice() {
                    if is_loose_constant_target(target, Some(a.value.as_ref()), self.allow) {
                        self.emit(stmt, &target.id);
                    }
                }
            }
            Stmt::AnnAssign(a) => {
                if let Expr::Name(target) = a.target.as_ref() {
                    if is_loose_constant_target(target, a.value.as_deref(), self.allow) {
                        self.emit(stmt, &target.id);
                    }
                }
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns `true` when `value` is a call whose callable resolves to
/// `TypeVar`, `ParamSpec`, `NewType`, or `TypeAliasType`, either bare
/// (*`TypeVar(...)`*) or attribute-qualified (*`typing.TypeVar(...)`*).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    fn rule() -> LooseConstants {
        LooseConstants {
            allow: HashSet::new(),
        }
    }

    fn lint(rule: &LooseConstants, src: &str) -> Vec<String> {
        rule.lint(&parse(src))
            .into_iter()
            .map(|d| d.message)
            .collect()
    }

    #[test]
    fn is_screaming_case_accepts_canonical_constants() {
        for id in ["PI", "MAX_RETRIES", "X1", "LOG_LEVEL_2"] {
            assert!(is_screaming_case(id), "expected screaming for {id}");
        }
    }

    #[test]
    fn is_screaming_case_rejects_mixed_and_lowercase_names() {
        for id in ["", "pi", "Pi", "pI", "_HIDDEN", "1ABC", "MAX_retries"] {
            assert!(!is_screaming_case(id), "expected not screaming for {id}");
        }
    }

    #[test]
    fn lint_flags_bare_screaming_case_assignment() {
        let messages = lint(&rule(), "PI = 3.14\n");
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("`PI`"));
    }

    #[test]
    fn lint_flags_screaming_case_ann_assign_with_and_without_value() {
        for src in ["X: int\n", "X: int = 1\n"] {
            assert_eq!(lint(&rule(), src).len(), 1, "source = {src:?}");
        }
    }

    #[test]
    fn lint_ignores_chained_and_tuple_targets() {
        for src in ["A = B = 1\n", "A, B = 1, 2\n", "FOO.bar = 1\n"] {
            assert!(
                lint(&rule(), src).is_empty(),
                "expected silence for {src:?}",
            );
        }
    }

    #[test]
    fn lint_respects_allowlist_entries() {
        let rule = LooseConstants {
            allow: HashSet::from(["LOG_LEVEL".to_owned()]),
        };
        assert!(lint(&rule, "LOG_LEVEL = 1\n").is_empty());
        assert_eq!(lint(&rule, "OTHER = 1\n").len(), 1);
    }

    #[test]
    fn lint_skips_assignments_inside_a_type_checking_block() {
        let src = "if TYPE_CHECKING:\n    PI = 3.14\n";
        assert!(lint(&rule(), src).is_empty());
    }

    #[test]
    fn lint_skips_assignments_inside_a_typing_dot_type_checking_block() {
        let src = "if typing.TYPE_CHECKING:\n    PI = 3.14\n";
        assert!(lint(&rule(), src).is_empty());
    }

    #[test]
    fn lint_skips_dunder_assignments() {
        for id in ["__version__", "__all__", "__author__"] {
            let src = format!("{id} = 1\n");
            assert!(lint(&rule(), &src).is_empty(), "source = {src:?}");
        }
    }

    #[test]
    fn lint_skips_function_local_screaming_case() {
        let src = "def f():\n    PI = 3.14\n    return PI\n";
        assert!(lint(&rule(), src).is_empty());
    }

    #[test]
    fn lint_skips_typing_constructor_calls() {
        for src in [
            "T = TypeVar(\"T\")\n",
            "P = ParamSpec(\"P\")\n",
            "UserId = NewType(\"UserId\", int)\n",
            "Vec = TypeAliasType(\"Vec\", list[int])\n",
            "T = typing.TypeVar(\"T\")\n",
        ] {
            assert!(lint(&rule(), src).is_empty(), "source = {src:?}");
        }
    }

    #[test]
    fn lint_walks_into_module_level_if_branches() {
        let src = "if sys.platform == \"win32\":\n    PI = 3.14\n";
        let messages = lint(&rule(), src);
        assert_eq!(messages.len(), 1, "messages = {messages:?}");
    }
}

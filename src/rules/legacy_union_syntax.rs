//! Recommends `X | Y` and `X | None` over `Union[X, Y]` and
//! `Optional[X]` when `target-version` is `3.10` or higher. Lint-only,
//! emits no edits.

use std::collections::HashMap;

use ruff_python_ast::name::{QualifiedName, UnqualifiedName};
use ruff_python_ast::token::parenthesized_range;
use ruff_python_ast::visitor::{walk_expr, walk_stmt, Visitor};
use ruff_python_ast::{AnyNodeRef, Expr, ExprSubscript, Identifier, PythonVersion, Stmt};
use ruff_text_size::Ranged;

use crate::config::Config;
use crate::diagnostics::Diagnostic;
use crate::primitives::binding::top_level_module;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct LegacyUnionSyntax {
    target_version: Option<PythonVersion>,
}

impl LegacyUnionSyntax {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            target_version: config.target_version,
        }
    }
}

impl Rule for LegacyUnionSyntax {
    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(LegacyUnionSyntax))
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        if self
            .target_version
            .is_none_or(|version| version < PythonVersion::PY310)
        {
            return Vec::new();
        }
        let imports = collect_typing_aliases(&source.ast().body);
        if imports.is_empty() {
            return Vec::new();
        }
        let mut walker = Walker {
            diagnostics: Vec::new(),
            imports: &imports,
            parents: Vec::new(),
            rule: self.id(),
            source,
        };
        walker.visit_body(&source.ast().body);
        walker.diagnostics
    }
}

#[derive(Copy, Clone)]
enum TypingForm {
    Optional,
    Union,
}

impl TypingForm {
    fn from_segments(segments: &[&str]) -> Option<Self> {
        match segments {
            ["typing" | "typing_extensions", "Optional"] => Some(Self::Optional),
            ["typing" | "typing_extensions", "Union"] => Some(Self::Union),
            _ => None,
        }
    }
}

/// Visits every `Subscript` and emits one diagnostic per match
/// against `typing.Optional` or `typing.Union`.
struct Walker<'a> {
    diagnostics: Vec<Diagnostic>,
    imports: &'a HashMap<&'a str, Vec<&'a str>>,
    parents: Vec<AnyNodeRef<'a>>,
    rule: RuleId,
    source: &'a Source,
}

impl<'a> Walker<'a> {
    fn maybe_emit(&mut self, subscript: &'a ExprSubscript) {
        let Some(qualified) = self.resolve(&subscript.value) else {
            return;
        };
        let Some(form) = TypingForm::from_segments(qualified.segments()) else {
            return;
        };
        let elements: &[Expr] = match subscript.slice.as_ref() {
            Expr::Tuple(tuple) => &tuple.elts,
            other => std::slice::from_ref(other),
        };
        let joined = elements
            .iter()
            .map(|e| self.source.slice(e))
            .collect::<Vec<_>>()
            .join(" | ");
        let modern = match form {
            TypingForm::Optional => format!("{joined} | None"),
            TypingForm::Union => joined,
        };
        let legacy = self.source.slice(subscript);
        let message = format!("`{legacy}` is the legacy form. Use `{modern}`");
        let parent = *self
            .parents
            .last()
            .expect("invariant: subscript visited inside a stmt or expr");
        let range = parenthesized_range(subscript.into(), parent, self.source.tokens())
            .unwrap_or(subscript.range());
        self.diagnostics
            .push(Diagnostic::lint(self.rule, range, message));
    }

    /// Resolves `value` (the head of a `Subscript`) to the qualified
    /// path it would name, given the module's `typing` aliases. `Opt`
    /// resolves to `typing.Optional`, `typing.Optional` resolves to
    /// `typing.Optional`, and an unresolved or non-`typing` head
    /// returns `None`.
    fn resolve(&self, value: &'a Expr) -> Option<QualifiedName<'a>> {
        let unqualified = UnqualifiedName::from_expr(value)?;
        let (head, tail) = unqualified.segments().split_first()?;
        let base = self.imports.get(head)?;
        Some(base.iter().copied().chain(tail.iter().copied()).collect())
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Subscript(subscript) = expr {
            self.maybe_emit(subscript);
        }
        self.parents.push(expr.into());
        walk_expr(self, expr);
        self.parents.pop();
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.parents.push(stmt.into());
        walk_stmt(self, stmt);
        self.parents.pop();
    }
}

/// Walks every top-level `Stmt::Import` and `Stmt::ImportFrom`,
/// recording the bound name and qualified path for each alias whose
/// source is `typing` or `typing_extensions`. Imports nested below
/// module scope are skipped.
fn collect_typing_aliases(body: &[Stmt]) -> HashMap<&str, Vec<&str>> {
    let mut imports: HashMap<&str, Vec<&str>> = HashMap::new();
    for stmt in body {
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    let name = alias.name.as_str();
                    if !is_typing_root(top_level_module(name)) {
                        continue;
                    }
                    let bound = alias.asname.as_ref().map_or(name, Identifier::as_str);
                    imports.insert(bound, name.split('.').collect());
                }
            }
            Stmt::ImportFrom(import) => {
                let Some(module) = import
                    .module
                    .as_ref()
                    .filter(|m| is_typing_root(m.as_str()))
                else {
                    continue;
                };
                for alias in &import.names {
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                    imports.insert(bound, vec![module.as_str(), alias.name.as_str()]);
                }
            }
            _ => {}
        }
    }
    imports
}

fn is_typing_root(module: &str) -> bool {
    matches!(module, "typing" | "typing_extensions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;
    use crate::test_support::parse;

    fn rule(version: Option<PythonVersion>) -> LegacyUnionSyntax {
        LegacyUnionSyntax {
            target_version: version,
        }
    }

    #[test]
    fn collects_aliased_from_import() {
        let source = parse("from typing import Optional as Opt\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(imports.get("Opt"), Some(&vec!["typing", "Optional"]));
    }

    #[test]
    fn collects_aliased_module_import() {
        let source = parse("import typing as t\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(imports.get("t"), Some(&vec!["typing"]));
    }

    #[test]
    fn collects_typing_extensions_alias() {
        let source = parse("from typing_extensions import Union\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("Union"),
            Some(&vec!["typing_extensions", "Union"]),
        );
    }

    #[test]
    fn empty_when_target_version_is_none() {
        let source = parse("from typing import Optional\nx: Optional[int]\n");
        assert!(rule(None).lint(&source).is_empty());
    }

    #[test]
    fn empty_when_target_version_below_310() {
        let source = parse("from typing import Optional\nx: Optional[int]\n");
        assert!(rule(Some(PythonVersion::PY39)).lint(&source).is_empty());
    }

    #[test]
    fn ignores_non_typing_imports() {
        let source = parse("from collections import OrderedDict\n");
        assert!(collect_typing_aliases(&source.ast().body).is_empty());
    }

    #[test]
    fn pins_severity_no_fix_and_message_format() {
        let source = parse("from typing import Optional\nx: Optional[int]\n");
        let diagnostics = rule(Some(PythonVersion::PY310)).lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.contains("`Optional[int]`"));
        assert!(only.message.contains("`int | None`"));
    }
}

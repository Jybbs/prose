//! Recommends `X | Y` and `X | None` over `Union[X, Y]` and
//! `Optional[X]` when `target-version` is `3.10` or higher. The
//! suggested rewrite rides as a display-only fix, recorded for the
//! reader but never applied.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Expr, ExprSubscript, Identifier, PythonVersion, Stmt,
    name::{QualifiedName, UnqualifiedName},
    visitor::{Visitor, walk_expr, walk_stmt},
};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{from_import_bound_name, top_level_module},
    rule::{Rule, RuleId},
    source::Source,
};

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
        Self::SLUG
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

/// Visits every `Subscript` and emits one diagnostic per match
/// against `typing.Optional` or `typing.Union`.
struct Walker<'a> {
    diagnostics: Vec<Diagnostic>,
    imports: &'a HashMap<&'a str, QualifiedName<'a>>,
    parents: Vec<AnyNodeRef<'a>>,
    rule: RuleId,
    source: &'a Source,
}

impl<'a> Walker<'a> {
    fn maybe_emit(&mut self, subscript: &'a ExprSubscript) {
        let Some(qualified) = self.resolve(&subscript.value) else {
            return;
        };
        let suffix = match qualified.segments() {
            ["typing" | "typing_extensions", "Optional"] => " | None",
            ["typing" | "typing_extensions", "Union"] => "",
            _ => return,
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
        let replacement = format!("{joined}{suffix}");
        let legacy = self.source.slice(subscript);
        let message = format!("`{legacy}` is the legacy form. Use `{replacement}`");
        let parent = *self
            .parents
            .last()
            .expect("invariant: subscript visited inside a stmt or expr");
        let range = self.source.paren_aware_range(subscript.into(), parent);
        let edit = Edit::range_replacement(replacement, range);
        self.diagnostics
            .push(Diagnostic::suggestion(self.rule, range, message, edit));
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
        Some(base.clone().extend_members(tail.iter().copied()))
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
fn collect_typing_aliases(body: &[Stmt]) -> HashMap<&str, QualifiedName<'_>> {
    let mut imports = HashMap::new();
    for stmt in body {
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    let name = alias.name.as_str();
                    if !is_typing_root(top_level_module(name)) {
                        continue;
                    }
                    let bound = alias.asname.as_ref().map_or(name, Identifier::as_str);
                    imports.insert(bound, QualifiedName::user_defined(name));
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
                    let bound = from_import_bound_name(alias);
                    imports.insert(
                        bound,
                        QualifiedName::user_defined(module.as_str())
                            .append_member(alias.name.as_str()),
                    );
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
    use ruff_diagnostics::Applicability;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::testing::parse;

    fn rule(version: Option<PythonVersion>) -> LegacyUnionSyntax {
        LegacyUnionSyntax {
            target_version: version,
        }
    }

    #[test]
    fn collects_aliased_from_import() {
        let source = parse("from typing import Optional as Opt\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("Opt").map(QualifiedName::segments),
            Some(["typing", "Optional"].as_slice()),
        );
    }

    #[test]
    fn collects_aliased_module_import() {
        let source = parse("import typing as t\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("t").map(QualifiedName::segments),
            Some(["typing"].as_slice()),
        );
    }

    #[test]
    fn collects_typing_extensions_alias() {
        let source = parse("from typing_extensions import Union\n");
        let imports = collect_typing_aliases(&source.ast().body);
        assert_eq!(
            imports.get("Union").map(QualifiedName::segments),
            Some(["typing_extensions", "Union"].as_slice()),
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
    fn pins_severity_display_only_fix_and_message_format() {
        let source = parse("from typing import Optional\nx: Optional[int]\n");
        let diagnostics = rule(Some(PythonVersion::PY310)).lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        let fix = only.fix.as_ref().expect("display-only suggestion");
        assert_eq!(fix.applicability(), Applicability::DisplayOnly);
        assert_eq!(fix.edits()[0].content(), Some("int | None"));
        assert!(only.message.contains("`Optional[int]`"));
        assert!(only.message.contains("`int | None`"));
    }

    #[test]
    fn rejects_non_typing_bare_import() {
        let source = parse("import os\n");
        assert!(collect_typing_aliases(&source.ast().body).is_empty());
    }
}

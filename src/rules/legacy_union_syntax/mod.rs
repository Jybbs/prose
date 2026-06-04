//! Recommends `X | Y` and `X | None` over `Union[X, Y]` and
//! `Optional[X]` when `target-version` is `3.10` or higher. The
//! suggested rewrite rides as a display-only fix, recorded for the
//! reader but never applied.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Expr, ExprSubscript, PythonVersion, Stmt,
    name::{QualifiedName, UnqualifiedName},
    visitor::{Visitor, walk_expr, walk_stmt},
};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::range::paren_aware_range,
    rule::{Rule, RuleId},
    source::Source,
};

mod aliases;

use aliases::collect_typing_aliases;

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
        let range = paren_aware_range(subscript.into(), parent, self.source.tokens());
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

#[cfg(test)]
mod tests {
    use ruff_diagnostics::Applicability;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::test_support::parse;

    fn rule(version: Option<PythonVersion>) -> LegacyUnionSyntax {
        LegacyUnionSyntax {
            target_version: version,
        }
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
}

//! Removes `from __future__ import annotations` when removal is
//! provably safe. The fix fires when the file has zero annotations,
//! when the configured target Python version defers annotations
//! per PEP 749, or when every name referenced by every annotation
//! is module-scope-defined before that annotation's offset.

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::{
    AnyParameterRef, Expr, PythonVersion, Stmt, StmtAnnAssign, StmtFunctionDef, StmtImportFrom,
};
use ruff_source_file::LineRanges;
use ruff_text_size::TextRange;

use crate::config::Config;
use crate::primitives::binding::BindingAnalysis;
use crate::primitives::edit::singleton_groups;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

const FUTURE_ANNOTATIONS: &str = "annotations";
const FUTURE_MODULE: &str = "__future__";

pub(crate) struct UnusedFutureAnnotations {
    target_version: Option<PythonVersion>,
}

impl UnusedFutureAnnotations {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            target_version: config.target_version,
        }
    }
}

impl Rule for UnusedFutureAnnotations {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let directives: Vec<(&StmtImportFrom, usize)> = source
            .ast()
            .body
            .iter()
            .filter_map(|stmt| {
                let node = stmt.as_import_from_stmt()?;
                Some((node, future_alias_index(node)?))
            })
            .collect();
        if directives.is_empty() || !rule_fires(source, self.target_version) {
            return Vec::new();
        }
        singleton_groups(
            directives
                .into_iter()
                .map(|(node, idx)| edit_for(source, node, idx)),
        )
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[derive(Default)]
struct AnnotationProbe {
    found: bool,
}

impl<'a> StatementVisitor<'a> for AnnotationProbe {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.found {
            return;
        }
        match stmt {
            Stmt::AnnAssign(_) => self.found = true,
            Stmt::FunctionDef(StmtFunctionDef {
                parameters,
                returns,
                ..
            }) if returns.is_some() || parameters.iter().any(|p| p.annotation().is_some()) => {
                self.found = true;
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

struct ResolutionChecker<'a> {
    all_safe: bool,
    analysis: &'a BindingAnalysis,
}

impl ResolutionChecker<'_> {
    fn check_annotation(&mut self, annotation: &Expr) {
        let unresolved = any_over_expr(annotation, &|expr: &Expr| {
            expr.as_name_expr().is_some_and(|name| {
                name.ctx.is_load()
                    && !self
                        .analysis
                        .is_defined_before(name.id.as_str(), name.range.start())
            })
        });
        if unresolved {
            self.all_safe = false;
        }
    }
}

impl<'b> StatementVisitor<'b> for ResolutionChecker<'_> {
    fn visit_stmt(&mut self, stmt: &'b Stmt) {
        if !self.all_safe {
            return;
        }
        match stmt {
            Stmt::AnnAssign(StmtAnnAssign { annotation, .. }) => {
                self.check_annotation(annotation);
            }
            Stmt::FunctionDef(StmtFunctionDef {
                parameters,
                returns,
                ..
            }) => {
                for annotation in parameters
                    .iter()
                    .filter_map(AnyParameterRef::annotation)
                    .chain(returns.as_deref())
                {
                    self.check_annotation(annotation);
                }
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

fn all_annotations_resolve_eagerly(source: &Source) -> bool {
    let mut checker = ResolutionChecker {
        all_safe: true,
        analysis: source.binding_analysis(),
    };
    checker.visit_body(&source.ast().body);
    checker.all_safe
}

fn edit_for(source: &Source, node: &StmtImportFrom, alias_idx: usize) -> Edit {
    if node.names.len() > 1 {
        Edit::range_deletion(surgical_alias_range(node, alias_idx))
    } else {
        Edit::range_deletion(source.text().full_lines_range(node.range))
    }
}

fn future_alias_index(node: &StmtImportFrom) -> Option<usize> {
    if node.level != 0 || node.module.as_deref() != Some(FUTURE_MODULE) {
        return None;
    }
    node.names
        .iter()
        .position(|alias| alias.name.id == FUTURE_ANNOTATIONS)
}

fn has_any_annotation(body: &[Stmt]) -> bool {
    let mut probe = AnnotationProbe::default();
    probe.visit_body(body);
    probe.found
}

fn rule_fires(source: &Source, target: Option<PythonVersion>) -> bool {
    !has_any_annotation(&source.ast().body)
        || target.is_some_and(PythonVersion::defers_annotations)
        || all_annotations_resolve_eagerly(source)
}

fn surgical_alias_range(node: &StmtImportFrom, alias_idx: usize) -> TextRange {
    let target = &node.names[alias_idx];
    match node.names.get(alias_idx + 1) {
        Some(next) => TextRange::new(target.range.start(), next.range.start()),
        None => TextRange::new(node.names[alias_idx - 1].range.end(), target.range.end()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    #[test]
    fn empty_file_emits_no_edits() {
        let source = parse("");
        let rule = UnusedFutureAnnotations::from_config(&Config::default());
        assert!(rule.apply(&source).is_empty());
    }

    #[test]
    fn target_py313_with_annotations_keeps_directive() {
        let source =
            parse("from __future__ import annotations\ndef f(x: int) -> int:\n    return x\n");
        let rule = UnusedFutureAnnotations::from_config(&Config {
            target_version: Some(PythonVersion::PY313),
            ..Config::default()
        });
        assert!(rule.apply(&source).is_empty());
    }
}

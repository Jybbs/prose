//! Removes `from __future__ import annotations` when removal is
//! provably safe. The fix fires when the file has zero annotations,
//! when the configured target Python version defers annotations
//! per PEP 749, or when every name referenced by every annotation
//! is module-scope-defined before that annotation's offset.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyParameterRef, Expr, PythonVersion, Stmt, StmtAnnAssign, StmtFunctionDef, StmtImportFrom,
    helpers::any_over_expr,
};
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    primitives::{
        binding::BindingAnalysis,
        edit::{singleton_groups, whole_line_deletion},
        imports::future_annotations_alias,
        walk::any_over_stmts,
    },
    rule::{Rule, RuleId},
    source::Source,
};

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
                Some((node, future_annotations_alias(node)?))
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

fn all_annotations_resolve_eagerly(source: &Source) -> bool {
    let analysis = source.binding_analysis();
    !any_over_stmts(&source.ast().body, |stmt| {
        statement_annotations(stmt)
            .iter()
            .any(|annotation| annotation_is_unresolved(annotation, analysis))
    })
}

/// True when `annotation` references a name no module-scope write
/// defines before it.
fn annotation_is_unresolved(annotation: &Expr, analysis: &BindingAnalysis) -> bool {
    any_over_expr(annotation, &|expr: &Expr| {
        expr.as_name_expr().is_some_and(|name| {
            name.ctx.is_load() && !analysis.is_defined_before(name.id.as_str(), name.range.start())
        })
    })
}

fn edit_for(source: &Source, node: &StmtImportFrom, alias_idx: usize) -> Edit {
    if node.names.len() > 1 {
        Edit::range_deletion(surgical_alias_range(node, alias_idx))
    } else {
        whole_line_deletion(source, node.range)
    }
}

fn has_any_annotation(body: &[Stmt]) -> bool {
    any_over_stmts(body, |stmt| !statement_annotations(stmt).is_empty())
}

fn rule_fires(source: &Source, target: Option<PythonVersion>) -> bool {
    !has_any_annotation(&source.ast().body)
        || target.is_some_and(PythonVersion::defers_annotations)
        || all_annotations_resolve_eagerly(source)
}

/// Yields each annotation expression a statement carries directly: an
/// annotated assignment's annotation, plus a function def's parameter
/// and return annotations.
fn statement_annotations(stmt: &Stmt) -> Vec<&Expr> {
    match stmt {
        Stmt::AnnAssign(StmtAnnAssign { annotation, .. }) => vec![&**annotation],
        Stmt::FunctionDef(StmtFunctionDef {
            parameters,
            returns,
            ..
        }) => parameters
            .iter()
            .filter_map(AnyParameterRef::annotation)
            .chain(returns.as_deref())
            .collect(),
        _ => Vec::new(),
    }
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
    use crate::testing::parse;

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

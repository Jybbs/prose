//! The pipeline's reparse- and compile-failure path and its error type.

use ruff_diagnostics::SourceMap;
use ruff_notebook::CellOffsets;
use ruff_python_ast::PythonVersion;
use ruff_python_parser::{ParseError, semantic_errors::SemanticSyntaxError};
use thiserror::Error;

use super::validity::first_semantic_error;
use crate::{primitives::edit::forward_offsets, rule::RuleId, source::Source};

/// Failure modes surfaced by the pipeline itself.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("rule `{rule}` produced output that did not compile: {error}")]
    Compile {
        error: SemanticSyntaxError,
        rule: RuleId,
    },
    #[error("rule `{rule}` produced output that did not parse")]
    Reparse {
        rule: RuleId,
        #[source]
        source: ParseError,
    },
}

/// Reparses `new_text`, sliding the source's cell offsets through `map`
/// so a notebook keeps current boundaries, and tags either failure with
/// the `rule` whose edits produced it. The semantic check runs only when
/// `gate` carries the version to evaluate against.
pub(super) fn reparse_or_reject(
    source: &Source,
    new_text: String,
    rule: RuleId,
    map: Option<SourceMap>,
    gate: Option<PythonVersion>,
) -> Result<Source, PipelineError> {
    let cell_offsets = map.map_or_else(CellOffsets::default, |m| {
        forward_offsets(source.cell_offsets(), &m)
    });
    let next = source
        .reparse_carrying(new_text, cell_offsets)
        .map_err(|source| PipelineError::Reparse { rule, source })?;
    if let Some(version) = gate
        && let Some(error) = first_semantic_error(&next, version)
    {
        return Err(PipelineError::Compile { error, rule });
    }
    Ok(next)
}

//! The pipeline's reparse-failure path and its error type.

use ruff_diagnostics::SourceMap;
use ruff_notebook::CellOffsets;
use ruff_python_parser::ParseError;
use thiserror::Error;

use crate::{primitives::edit::forward_offsets, rule::RuleId, source::Source};

/// Failure modes surfaced by the pipeline itself.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("rule `{rule}` produced output that did not parse")]
    Reparse {
        rule: RuleId,
        #[source]
        source: ParseError,
    },
}

/// Reparses `new_text`, sliding the source's cell offsets through `map`
/// so a notebook keeps current boundaries, and tags a parse failure with
/// the `rule` whose edits produced it.
pub(super) fn reparse_or_reject(
    source: &Source,
    new_text: String,
    rule: RuleId,
    map: Option<SourceMap>,
) -> Result<Source, PipelineError> {
    let cell_offsets = map.map_or_else(CellOffsets::default, |m| {
        forward_offsets(source.cell_offsets(), &m)
    });
    source
        .reparse_carrying(new_text, cell_offsets)
        .map_err(|source| PipelineError::Reparse { rule, source })
}

//! The pipeline's reparse-failure path and its error type.

use ruff_notebook::CellOffsets;
use ruff_python_parser::ParseError;
use thiserror::Error;

use crate::{rule::RuleId, source::Source};

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

/// Reparses `new_text` carrying `cell_offsets` forward, tagging a parse
/// failure with the `rule` whose edits produced it.
pub(super) fn reparse_or_reject(
    source: &Source,
    new_text: String,
    rule: RuleId,
    cell_offsets: CellOffsets,
) -> Result<Source, PipelineError> {
    source
        .reparse_carrying(new_text, cell_offsets)
        .map_err(|source| PipelineError::Reparse { rule, source })
}

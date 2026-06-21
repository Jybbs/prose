//! The pipeline's reparse-failure path and its error type.

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

/// Reparses `new_text`, tagging a parse failure with the `rule` whose
/// edits produced it.
pub(super) fn reparse_or_reject(
    source: &Source,
    new_text: String,
    rule: RuleId,
) -> Result<Source, PipelineError> {
    source
        .reparse(new_text)
        .map_err(|source| PipelineError::Reparse { rule, source })
}

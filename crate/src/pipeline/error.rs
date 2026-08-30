//! The pipeline's reparse- and compile-failure path and its error type.

use ruff_diagnostics::SourceMap;
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::{ParseError, ParseOptions, parse, semantic_errors::SemanticSyntaxError};
use ruff_source_file::OneIndexed;
use ruff_text_size::TextLen;
use thiserror::Error;

use super::validity::first_semantic_error;
use crate::{
    primitives::edit::forward_offsets,
    rule::{Rule, RuleId},
    source::Source,
};

/// Failure modes surfaced by the pipeline itself.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("rule `{rule}` left notebook cell {cell} unparseable")]
    Cell {
        cell: OneIndexed,
        rule: RuleId,
        #[source]
        source: ParseError,
    },
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
/// so a notebook keeps current boundaries, carrying into the result the
/// tables `rule` declares its edits leave standing, and tagging each
/// failure with `rule`. A cell the source split cleanly is checked on
/// its own through [`reject_split_cell`], and the semantic check runs
/// only when `gate` carries the version to evaluate against.
pub(super) fn reparse_or_reject(
    source: Source,
    new_text: String,
    rule: &dyn Rule,
    map: &SourceMap,
    gate: Option<PythonVersion>,
) -> Result<Source, PipelineError> {
    let id = rule.id();
    let limit = new_text.text_len();
    let cell_offsets = forward_offsets(source.cell_offsets(), map, limit);
    let mut next = source
        .reparse_carrying(new_text, cell_offsets)
        .map_err(|source| PipelineError::Reparse { rule: id, source })?;
    reject_split_cell(&source, &next, id)?;
    if let Some(version) = gate
        && let Some(error) = first_semantic_error(&next, version)
    {
        return Err(PipelineError::Compile { error, rule: id });
    }
    next.inherit(source, map, rule.preserves());
    Ok(next)
}

/// Parses each of `after`'s notebook cells on its own, tagging the first
/// failure with its absolute notebook cell number and `rule`. Only a cell
/// `before` split cleanly at both boundaries is checked, and an ordinary
/// module holds no cell boundary and passes through.
fn reject_split_cell(before: &Source, after: &Source, rule: RuleId) -> Result<(), PipelineError> {
    if !after.is_notebook() {
        return Ok(());
    }
    for (index, text) in after.cell_texts().into_iter().enumerate() {
        if before.cell_splits_cleanly(index) && before.cell_splits_cleanly(index + 1) {
            parse(text, ParseOptions::from(PySourceType::Ipynb)).map_err(|source| {
                PipelineError::Cell {
                    cell: after.cell_number(index),
                    rule,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::testing::{notebook, parse};

    fn rule() -> RuleId {
        RuleId::from("breaks-parse")
    }

    #[test]
    fn reject_split_cell_accepts_cells_that_still_parse() {
        let clean = notebook(&["x = 1\n", "y = 2\n"]);

        assert_matches!(reject_split_cell(&clean, &clean, rule()), Ok(()));
    }

    #[test]
    fn reject_split_cell_holds_a_cell_the_source_already_split_mid_statement() {
        let split = notebook(&["def helper():", "    return 1\n"]);

        assert_matches!(reject_split_cell(&split, &split, rule()), Ok(()));
    }

    #[test]
    fn reject_split_cell_names_the_cell_that_stopped_parsing() {
        let before = notebook(&["x = 1\n", "y = 2\n"]);
        let after = notebook(&["def helper():", "    return 1\n"]);

        assert_matches!(
            reject_split_cell(&before, &after, rule()),
            Err(PipelineError::Cell { cell, rule, .. })
                if cell.get() == 1 && rule.as_str() == "breaks-parse"
        );
    }

    #[test]
    fn reject_split_cell_passes_an_ordinary_module_through() {
        let module = parse("x = 1\n");

        assert_matches!(reject_split_cell(&module, &module, rule()), Ok(()));
    }
}

//! The pipeline's reparse-failure path and its error type.

use ruff_diagnostics::SourceMap;
use ruff_notebook::CellOffsets;
use ruff_python_ast::PySourceType;
use ruff_python_parser::{ParseError, ParseOptions, parse};
use ruff_source_file::OneIndexed;
use thiserror::Error;

use crate::{primitives::edit::forward_offsets, rule::RuleId, source::Source};

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
    #[error("rule `{rule}` produced output that did not parse")]
    Reparse {
        rule: RuleId,
        #[source]
        source: ParseError,
    },
}

/// Reparses `new_text`, sliding the source's cell offsets through `map`
/// so a notebook keeps current boundaries, and tags a parse failure with
/// the `rule` whose edits produced it. A notebook cell that split the
/// source's statements before the rule ran is checked past the
/// whole-buffer reparse, so a split leaving it unparseable is rejected
/// rather than written out.
pub(super) fn reparse_or_reject(
    source: &Source,
    new_text: String,
    rule: RuleId,
    map: Option<SourceMap>,
) -> Result<Source, PipelineError> {
    let cell_offsets = map.map_or_else(CellOffsets::default, |m| {
        forward_offsets(source.cell_offsets(), &m)
    });
    let next = source
        .reparse_carrying(new_text, cell_offsets)
        .map_err(|source| PipelineError::Reparse { rule, source })?;
    reject_split_cell(source, &next, rule)?;
    Ok(next)
}

/// Parses each of `after`'s notebook cells on its own, tagging the first
/// failure with its one-indexed cell number and `rule`. Only a cell whose
/// boundaries both split `before`'s statements is checked, so one the
/// author already wrote through a statement is left as it stands. An
/// ordinary module holds no cell boundary and passes through.
fn reject_split_cell(before: &Source, after: &Source, rule: RuleId) -> Result<(), PipelineError> {
    if !after.is_notebook() {
        return Ok(());
    }
    for (index, text) in after.cell_texts().into_iter().enumerate() {
        if before.cell_splits_cleanly(index) && before.cell_splits_cleanly(index + 1) {
            parse(text, ParseOptions::from(PySourceType::Ipynb)).map_err(|source| {
                PipelineError::Cell {
                    cell: OneIndexed::from_zero_indexed(index),
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

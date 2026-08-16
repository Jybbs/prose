//! Notebook formatting: parse an `.ipynb`, run the pipeline once over
//! its concatenated code cells, and re-emit the JSON with outputs,
//! metadata, and structure preserved.

use itertools::Itertools;
use ruff_diagnostics::SourceMap;
use ruff_notebook::{CellOffsets, Notebook, NotebookIndex};
use ruff_source_file::SourceFileBuilder;

use super::{
    FileOutcome, Pass,
    process::{drive, failed},
    resolve::Resolved,
};
use crate::{cache::Rewrite, cli::exit_status::ExitStatus, source::Source};

/// Reparses `written`, the JSON a notebook rewrite lands on disk, back
/// into a `Source`, so a caller reads the cells that file will carry
/// rather than the concatenation they were serialized from. The two
/// diverge where `Notebook::update` cuts a cell at a different boundary
/// than the run carried.
pub(super) fn as_written(written: &str, name: &str) -> Option<Source> {
    let notebook = Notebook::from_source_code(written).ok()?;
    Source::from_notebook(&notebook, name).ok()
}

/// Parses `text` as a notebook and runs `pass` over its code cells. A
/// non-Python notebook is passed over clean, and a read or parse
/// failure surfaces at the parse-error status.
pub(super) fn process(text: String, name: String, resolved: &Resolved, pass: Pass) -> FileOutcome {
    let notebook = match Notebook::from_source_code(&text) {
        Ok(notebook) => notebook,
        Err(e) => {
            return failed(
                ExitStatus::ParseError,
                format_args!("notebook error in `{name}`: {e}"),
            );
        }
    };
    if !notebook.is_python_notebook() {
        let file = SourceFileBuilder::new(name, text).finish();
        return FileOutcome::Done {
            cached: false,
            diagnostics: Vec::new(),
            file,
            notebook_index: None,
            rewrite: Rewrite::Skipped,
            unstable: None,
        };
    }
    match Source::from_notebook(&notebook, name.as_str()) {
        Ok(source) => run(source, notebook, resolved, pass),
        Err(e) => failed(
            ExitStatus::ParseError,
            format_args!("parse error in `{name}`: {e}"),
        ),
    }
}

/// Returns the concatenated code-cell source of a notebook paired with
/// its cell index, the text a cache hit rebuilds its diagnostics file
/// from and the translator it renders cell-relative positions through.
pub(super) fn rehydrated(text: &str) -> Option<(String, NotebookIndex)> {
    Notebook::from_source_code(text).ok().map(|notebook| {
        let source = notebook.source_code().to_owned();
        (source, notebook.into_index())
    })
}

/// Builds the notebook rewrite, sliding the cell offsets against the
/// run's deltas before re-emitting the JSON.
fn build_rewrite(
    notebook: &mut Notebook,
    original_offsets: &CellOffsets,
    original_code: &str,
    formatted: &Source,
) -> Rewrite {
    let formatted_code = formatted.text();
    if formatted_code == original_code {
        return Rewrite::Unchanged;
    }
    let final_offsets = formatted.cell_offsets();
    let mut update_map = SourceMap::default();
    for (&original, &updated) in original_offsets.iter().zip_eq(final_offsets.iter()) {
        update_map.push_marker(original, updated);
    }
    notebook.update(&update_map, formatted_code.to_owned());
    let before = slice_cells(original_code, original_offsets);
    let after = slice_cells(formatted_code, final_offsets);
    Rewrite::notebook(before, after, emit(notebook))
}

/// Serializes `notebook` back to its JSON document.
fn emit(notebook: &Notebook) -> String {
    let mut bytes = Vec::new();
    notebook
        .write(&mut bytes)
        .expect("re-emitting a parsed notebook to memory cannot fail");
    String::from_utf8(bytes).expect("notebook JSON is valid UTF-8")
}

/// Runs the notebook's concatenated source through the pipeline,
/// building the notebook rewrite from the formatted result. The cell
/// index built off the original cells threads through to the reporter so
/// it renders each diagnostic against its own cell.
fn run(source: Source, mut notebook: Notebook, resolved: &Resolved, pass: Pass) -> FileOutcome {
    let index = notebook.index().clone();
    let original_offsets = source.cell_offsets().clone();
    let original_code = source.text().to_owned();
    drive(
        source,
        resolved,
        pass,
        Some(index),
        move |formatted, _file| {
            build_rewrite(&mut notebook, &original_offsets, &original_code, formatted)
        },
    )
}

/// Splits `code` into its per-cell sources at `offsets`.
fn slice_cells(code: &str, offsets: &CellOffsets) -> Vec<String> {
    offsets
        .ranges()
        .map(|range| code[range].to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::notebook;

    #[test]
    fn rehydrated_returns_none_for_malformed_json() {
        assert!(rehydrated("{not json").is_none());
    }

    #[test]
    fn slice_cells_splits_each_cell_at_its_boundary() {
        let source = notebook(&["a = 1\n", "b = 2\n"]);

        let cells = slice_cells(source.text(), source.cell_offsets());
        assert_eq!(cells, vec!["a = 1\n\n".to_owned(), "b = 2\n\n".to_owned()]);
    }
}

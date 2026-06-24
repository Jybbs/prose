//! Notebook formatting: parse an `.ipynb`, run the pipeline once over
//! its concatenated code cells, and re-emit the JSON with outputs,
//! metadata, and structure preserved.

use ruff_diagnostics::SourceMap;
use ruff_notebook::{Notebook, NotebookIndex};
use ruff_source_file::SourceFileBuilder;
use ruff_text_size::{TextRange, TextSize};

use super::process::{diagnose_only, failed, run_and_assemble};
use super::{FileOutcome, Pass};
use crate::{cache::Rewrite, cli::exit_status::ExitStatus, pipeline::Pipeline, source::Source};

/// Builds the notebook rewrite, sliding the cell offsets against the
/// run's deltas before re-emitting the JSON.
fn build_rewrite(
    notebook: &mut Notebook,
    original_offsets: &[TextSize],
    original_code: &str,
    formatted: &Source,
) -> Rewrite {
    let formatted_code = formatted.text();
    if formatted_code == original_code {
        return Rewrite::Unchanged;
    }
    let final_offsets = formatted.cell_offsets();
    let mut update_map = SourceMap::default();
    for (&original, &updated) in original_offsets.iter().zip(final_offsets.iter()) {
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

/// Parses `text` as a notebook and runs `pass` over its code cells. A
/// non-Python notebook is passed over clean, and a read or parse
/// failure surfaces at the parse-error status.
pub(super) fn process(text: String, name: String, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
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
        };
    }
    match Source::from_notebook(&notebook, name.as_str()) {
        Ok(source) => run(source, notebook, pipeline, pass),
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

/// Runs the notebook's concatenated source through the pipeline,
/// building the notebook rewrite from the formatted result. The cell
/// index built off the original cells threads through to the reporter so
/// it renders each diagnostic against its own cell.
fn run(source: Source, mut notebook: Notebook, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let index = notebook.index().clone();
    if let Pass::Diagnose { validate } = pass {
        return diagnose_only(source, pipeline, validate, Some(index));
    }
    let original_offsets: Box<[TextSize]> = source.cell_offsets().iter().copied().collect();
    let original_code = source.text().to_owned();
    run_and_assemble(
        source,
        pipeline,
        matches!(pass, Pass::Both),
        Some(index),
        move |formatted, _file| {
            build_rewrite(&mut notebook, &original_offsets, &original_code, formatted)
        },
    )
}

/// Splits `code` into its per-cell sources at `offsets`.
fn slice_cells(code: &str, offsets: &[TextSize]) -> Vec<String> {
    offsets
        .windows(2)
        .map(|pair| code[TextRange::new(pair[0], pair[1])].to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rehydrated_returns_none_for_malformed_json() {
        assert!(rehydrated("{not json").is_none());
    }

    #[test]
    fn slice_cells_splits_each_cell_at_its_boundary() {
        let cells = slice_cells(
            "a\nb\n",
            &[TextSize::new(0), TextSize::new(2), TextSize::new(4)],
        );
        assert_eq!(cells, vec!["a\n".to_owned(), "b\n".to_owned()]);
    }
}

//! Diagnostic model and output emitters.

use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use ruff_notebook::NotebookIndex;
use ruff_source_file::{LineColumn, SourceFile};
use ruff_text_size::TextRange;
use serde::Serialize;

use crate::rule::RuleId;

pub(crate) mod github;
pub(crate) mod json;
pub(crate) mod model;
pub(crate) mod sarif;
pub(crate) mod text;

pub use json::lint_records_json;
pub use model::{Diagnostic, Severity};

pub(crate) use github::Github;
pub(crate) use json::Json;
pub(crate) use sarif::Sarif;
pub(crate) use text::Text;

pub(crate) trait Emitter {
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        summary: &EmitterSummary,
    ) -> io::Result<()>;
}

/// Run-wide rollup accumulated across every processed file. Feeds both
/// the JSON envelope's closing record and the human run summary.
#[derive(Default)]
pub(crate) struct EmitterSummary {
    pub(crate) diagnostics_total: usize,
    pub(crate) files_changed: usize,
    pub(crate) files_visited: usize,
    pub(crate) files_with_diagnostics: usize,
    pub(crate) rules_fired: BTreeMap<RuleId, usize>,
}

/// One file's diagnostics paired with the `SourceFile` they range into
/// and, for a notebook, the `NotebookIndex` translating a concatenated
/// position into a cell-relative one. The translator threads through
/// this one seam rather than each emitter rebuilding it, modeled on
/// ruff's `EmitterContext`.
pub(crate) struct Run<'a> {
    pub(crate) diagnostics: &'a [Diagnostic],
    pub(crate) file: &'a SourceFile,
    pub(crate) notebook_index: Option<&'a NotebookIndex>,
}

impl<'a> Run<'a> {
    pub(crate) fn new(
        file: &'a SourceFile,
        diagnostics: &'a [Diagnostic],
        notebook_index: Option<&'a NotebookIndex>,
    ) -> Self {
        Self {
            diagnostics,
            file,
            notebook_index,
        }
    }
}

/// Flattens every run into its `(file, notebook index, diagnostic)`
/// triples in file-major order, the traversal each emitter walks. The
/// notebook index is `None` for an ordinary module.
fn diagnostics<'a>(
    runs: &'a [Run<'a>],
) -> impl Iterator<Item = (&'a SourceFile, Option<&'a NotebookIndex>, &'a Diagnostic)> {
    runs.iter().flat_map(|run| {
        run.diagnostics
            .iter()
            .map(move |d| (run.file, run.notebook_index, d))
    })
}

fn line_columns(file: &SourceFile, range: TextRange) -> (LineColumn, LineColumn) {
    let code = file.to_source_code();
    (
        code.line_column(range.start()),
        code.line_column(range.end()),
    )
}

fn write_json_line<T: Serialize>(writer: &mut dyn Write, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

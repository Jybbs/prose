//! Output emitters rendering diagnostics in each CLI output format.

use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use ruff_notebook::NotebookIndex;
use ruff_source_file::SourceFile;
use serde::Serialize;

use crate::{diagnostics::Diagnostic, rule::RuleId};

mod github;
mod json;
mod sarif;
mod text;

pub(super) use github::Github;
pub(super) use json::Json;
pub(super) use sarif::Sarif;
pub(super) use text::Text;

pub(super) trait Emitter {
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
pub(super) struct EmitterSummary {
    pub(super) diagnostics_total: usize,
    pub(super) files_changed: usize,
    pub(super) files_visited: usize,
    pub(super) files_with_diagnostics: usize,
    pub(super) lint_total: usize,
    pub(super) rules_fired: BTreeMap<RuleId, usize>,
}

/// One file's diagnostics paired with the `SourceFile` they range into
/// and, for a notebook, the `NotebookIndex` translating a concatenated
/// position into a cell-relative one. The translator threads through
/// this one seam rather than each emitter rebuilding it, modeled on
/// ruff's `EmitterContext`.
pub(super) struct Run<'a> {
    pub(super) diagnostics: &'a [Diagnostic],
    pub(super) file: &'a SourceFile,
    pub(super) notebook_index: Option<&'a NotebookIndex>,
}

impl<'a> Run<'a> {
    pub(super) fn new(
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
/// triples in file-major order, the traversal the structured emitters
/// walk. The notebook index is `None` for an ordinary module.
fn diagnostics<'a>(
    runs: &'a [Run<'a>],
) -> impl Iterator<Item = (&'a SourceFile, Option<&'a NotebookIndex>, &'a Diagnostic)> {
    runs.iter().flat_map(|run| {
        run.diagnostics
            .iter()
            .map(move |d| (run.file, run.notebook_index, d))
    })
}

fn write_json_line<T: Serialize>(writer: &mut dyn Write, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")
}

#[cfg(test)]
fn emitted(
    emitter: &dyn Emitter,
    file: &SourceFile,
    diagnostics: &[Diagnostic],
    summary: &EmitterSummary,
) -> Vec<u8> {
    emitted_runs(emitter, &[Run::new(file, diagnostics, None)], summary)
}

#[cfg(test)]
fn emitted_runs(emitter: &dyn Emitter, runs: &[Run<'_>], summary: &EmitterSummary) -> Vec<u8> {
    let mut buf = Vec::new();
    emitter.emit(&mut buf, runs, summary).expect("emits");
    buf
}

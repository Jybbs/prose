//! Output emitters rendering diagnostics in each CLI output format.

use std::{
    collections::BTreeMap,
    fmt,
    io::{self, Write},
};

use ruff_notebook::NotebookIndex;
use ruff_source_file::{LineColumn, OneIndexed, SourceFile};
use ruff_text_size::Ranged;
use serde::Serialize;

use crate::{
    diagnostics::Diagnostic,
    findings::located,
    rules::RuleId,
    source::{Refusal, Target},
};

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
    pub(super) refused: Vec<RefusedEntry>,
    pub(super) rules_fired: BTreeMap<RuleId, usize>,
    pub(super) unstable: Vec<UnstableEntry>,
}

/// One file whose syntax its target rejects, carried into the structured
/// summaries at the position the refusal names, which inside a notebook
/// is relative to its `cell`.
#[derive(Clone, Debug)]
pub(super) struct RefusedEntry {
    pub(super) cell: Option<OneIndexed>,
    pub(super) end: LineColumn,
    pub(super) file: String,
    pub(super) message: String,
    pub(super) start: LineColumn,
}

impl RefusedEntry {
    /// Builds the entry `refusal` names in `file`, its message naming
    /// `target`, translating the position through `index` for a
    /// notebook.
    pub(super) fn new(
        file: &SourceFile,
        index: Option<&NotebookIndex>,
        refusal: &Refusal,
        target: Target,
    ) -> Self {
        let (start, end, cell) = located(file, index, refusal.range());
        Self {
            cell,
            end,
            file: file.name().to_owned(),
            message: refusal.message(target),
            start,
        }
    }
}

/// Renders the stderr line a refused file prints, naming the file, the
/// cell inside a notebook, and the line and column the refusal opens at.
impl fmt::Display for RefusedEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "syntax error in `{}`, ", self.file)?;
        if let Some(cell) = self.cell {
            write!(f, "cell {cell}, ")?;
        }
        write!(
            f,
            "line {}, column {}: {}",
            self.start.line, self.start.column, self.message,
        )
    }
}

/// One file's diagnostics paired with the `SourceFile` they range into
/// and, for a notebook, the `NotebookIndex` translating a concatenated
/// position into a cell-relative one.
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

/// One file whose rewrite a second run would change, carried into the
/// structured summaries.
#[derive(serde::Serialize)]
pub(super) struct UnstableEntry {
    pub(super) file: String,
    pub(super) rules: Vec<RuleId>,
}

pub(in crate::cli) fn write_json_line<T: Serialize>(
    writer: &mut dyn Write,
    value: &T,
) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")
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
fn emitted_string(
    emitter: &dyn Emitter,
    file: &SourceFile,
    diagnostics: &[Diagnostic],
    summary: &EmitterSummary,
) -> String {
    String::from_utf8(emitted(emitter, file, diagnostics, summary)).expect("utf-8")
}

#[cfg(test)]
fn emitted_runs(emitter: &dyn Emitter, runs: &[Run<'_>], summary: &EmitterSummary) -> Vec<u8> {
    let mut buf = Vec::new();
    emitter.emit(&mut buf, runs, summary).expect("emits");
    buf
}

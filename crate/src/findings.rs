//! The lint-diagnostic JSON records the docs site decorates onto its
//! formatted view. The CLI json emitter renders them inside its NDJSON
//! stream, and the wasm bindings return them beside the formatted text.

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_notebook::NotebookIndex;
use ruff_source_file::{LineColumn, OneIndexed, SourceFile};
use ruff_text_size::{Ranged, TextRange};
use serde::Serialize;

use crate::diagnostics::Diagnostic;

#[derive(Serialize)]
pub(crate) struct JsonDiagnostic<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    cell: Option<OneIndexed>,
    code: &'a str,
    end_location: JsonLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<&'a str>,
    fix: Option<JsonFix<'a>>,
    location: JsonLocation,
    message: &'a str,
}

impl<'a> JsonDiagnostic<'a> {
    pub(crate) fn new(
        file: &'a SourceFile,
        index: Option<&NotebookIndex>,
        diag: &'a Diagnostic,
        full: bool,
    ) -> Self {
        let (start, end, cell) = located(file, index, diag.range);
        Self {
            cell,
            code: diag.rule.as_str(),
            end_location: end.into(),
            filename: full.then(|| file.name()),
            fix: diag
                .fix
                .as_ref()
                .map(|fix| JsonFix::new(file, index, fix, full)),
            location: start.into(),
            message: &diag.message,
        }
    }
}

#[derive(Serialize)]
struct JsonEdit<'a> {
    before: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_location: Option<JsonLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<JsonLocation>,
}

impl<'a> JsonEdit<'a> {
    fn new(
        file: &'a SourceFile,
        index: Option<&NotebookIndex>,
        edit: &'a Edit,
        full: bool,
    ) -> Self {
        let (location, end_location) = full
            .then(|| located(file, index, edit.range()))
            .map(|(start, end, _)| (JsonLocation::from(start), JsonLocation::from(end)))
            .unzip();
        Self {
            before: &file.source_text()[edit.range()],
            content: edit.content().unwrap_or_default(),
            end_location,
            location,
        }
    }
}

#[derive(Serialize)]
struct JsonFix<'a> {
    applicability: Applicability,
    edits: Vec<JsonEdit<'a>>,
}

impl<'a> JsonFix<'a> {
    fn new(file: &'a SourceFile, index: Option<&NotebookIndex>, fix: &'a Fix, full: bool) -> Self {
        Self {
            applicability: fix.applicability(),
            edits: fix
                .edits()
                .iter()
                .map(|edit| JsonEdit::new(file, index, edit, full))
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct JsonLocation {
    column: OneIndexed,
    row: OneIndexed,
}

impl From<LineColumn> for JsonLocation {
    fn from(LineColumn { line, column }: LineColumn) -> Self {
        Self { column, row: line }
    }
}

pub(crate) fn line_columns(file: &SourceFile, range: TextRange) -> (LineColumn, LineColumn) {
    let code = file.to_source_code();
    (
        code.line_column(range.start()),
        code.line_column(range.end()),
    )
}

/// Renders the lint-severity diagnostics as the JSON records the docs
/// site reads, or `None` when the run emitted none.
pub fn lint_records_json(file: &SourceFile, diagnostics: &[Diagnostic]) -> Option<String> {
    let records: Vec<JsonDiagnostic> = diagnostics
        .iter()
        .filter(|diag| diag.severity.is_lint())
        .map(|diag| JsonDiagnostic::new(file, None, diag, false))
        .collect();
    (!records.is_empty())
        .then(|| serde_json::to_string_pretty(&records).expect("lint records serialize"))
}

/// The start and end positions of `range` plus, for a notebook, the
/// absolute cell holding it. A notebook translates the positions to
/// cell-relative coordinates through the index, where a module leaves
/// them absolute with no cell.
fn located(
    file: &SourceFile,
    index: Option<&NotebookIndex>,
    range: TextRange,
) -> (LineColumn, LineColumn, Option<OneIndexed>) {
    let (start, end) = line_columns(file, range);
    match index {
        Some(index) => (
            index.translate_line_column(&start),
            index.translate_line_column(&end),
            index.cell(start.line),
        ),
        None => (start, end, None),
    }
}

//! Diagnostic model and output emitters.

use std::collections::BTreeMap;
use std::io::{self, Write};

use ruff_source_file::{LineColumn, SourceFile};
use ruff_text_size::TextRange;
use serde::Serialize;

use crate::rule::RuleId;

pub(crate) mod github;
pub(crate) mod json;
pub(crate) mod model;
pub(crate) mod sarif;
pub(crate) mod text;

pub use model::{Diagnostic, Severity};

pub(crate) use github::Github;
pub(crate) use json::Json;
pub(crate) use sarif::Sarif;
pub(crate) use text::Text;

/// One pipeline run paired with the diagnostics it produced.
pub(crate) type Run<'a> = (&'a SourceFile, &'a [Diagnostic]);

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

pub(crate) fn line_columns(file: &SourceFile, range: TextRange) -> (LineColumn, LineColumn) {
    let code = file.to_source_code();
    (
        code.line_column(range.start()),
        code.line_column(range.end()),
    )
}

pub(crate) fn write_json_line<T: Serialize>(writer: &mut dyn Write, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

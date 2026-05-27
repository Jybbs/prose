//! Diagnostic model and output emitters.

use std::io::{self, Write};

use ruff_source_file::{LineColumn, SourceFile};
use ruff_text_size::TextRange;

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
    fn emit(&self, writer: &mut dyn Write, runs: &[Run<'_>]) -> io::Result<()>;
}

pub(crate) fn line_columns(file: &SourceFile, range: TextRange) -> (LineColumn, LineColumn) {
    let code = file.to_source_code();
    (
        code.line_column(range.start()),
        code.line_column(range.end()),
    )
}

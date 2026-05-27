//! Json emitter: NDJSON of Ruff-shaped diagnostic records.

use std::io::{self, Write};

use ruff_diagnostics::{Applicability, Edit};
use ruff_source_file::{LineColumn, OneIndexed, SourceFile};
use ruff_text_size::Ranged;
use serde::Serialize;

use crate::diagnostics::{line_columns, Diagnostic, Emitter, Run};

pub(crate) struct Json;

impl Emitter for Json {
    fn emit(&self, writer: &mut dyn Write, runs: &[Run<'_>]) -> io::Result<()> {
        for (file, diagnostics) in runs {
            for diag in *diagnostics {
                serde_json::to_writer(&mut *writer, &JsonDiagnostic::new(file, diag))
                    .map_err(io::Error::other)?;
                writer.write_all(b"\n")?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    code: &'a str,
    end_location: JsonLocation,
    filename: &'a str,
    fix: Option<JsonFix<'a>>,
    location: JsonLocation,
    message: &'a str,
}

impl<'a> JsonDiagnostic<'a> {
    fn new(file: &'a SourceFile, diag: &'a Diagnostic) -> Self {
        let (start, end) = line_columns(file, diag.range);
        Self {
            code: diag.rule.as_str(),
            end_location: end.into(),
            filename: file.name(),
            fix: diag.fix.as_ref().map(|edit| JsonFix::new(file, edit)),
            location: start.into(),
            message: &diag.message,
        }
    }
}

#[derive(Serialize)]
struct JsonEdit<'a> {
    content: &'a str,
    end_location: JsonLocation,
    location: JsonLocation,
}

impl<'a> JsonEdit<'a> {
    fn new(file: &'a SourceFile, edit: &'a Edit) -> Self {
        let (start, end) = line_columns(file, edit.range());
        Self {
            content: edit.content().unwrap_or_default(),
            end_location: end.into(),
            location: start.into(),
        }
    }
}

#[derive(Serialize)]
struct JsonFix<'a> {
    applicability: Applicability,
    edits: Vec<JsonEdit<'a>>,
}

impl<'a> JsonFix<'a> {
    fn new(file: &'a SourceFile, edit: &'a Edit) -> Self {
        Self {
            applicability: Applicability::Safe,
            edits: vec![JsonEdit::new(file, edit)],
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

#[cfg(test)]
mod tests {
    use ruff_text_size::TextRange;
    use serde_json::Value;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::RuleId;
    use crate::source::Source;

    fn diag() -> Diagnostic {
        let range = TextRange::new(0.into(), 1.into());
        Diagnostic {
            fix: Some(Edit::range_replacement("y".to_owned(), range)),
            message: "rewrite x to y".to_owned(),
            range,
            rule: RuleId::from("rewrite-x"),
            severity: Severity::Format,
        }
    }

    #[test]
    fn emits_one_ndjson_record_per_diagnostic() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = diag();
        let mut buf = Vec::<u8>::new();
        Json.emit(
            &mut buf,
            &[(source.source_file(), std::slice::from_ref(&diag))],
        )
        .expect("emits");
        let text = String::from_utf8(buf).expect("utf-8");
        assert!(text.ends_with('\n'));
        assert_eq!(text.matches('\n').count(), 1);
        let _: Value = serde_json::from_str(text.trim_end()).expect("parses as JSON");
    }

    #[test]
    fn populates_fix_payload_with_safe_applicability_and_edit() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = diag();
        let mut buf = Vec::<u8>::new();
        Json.emit(
            &mut buf,
            &[(source.source_file(), std::slice::from_ref(&diag))],
        )
        .expect("emits");
        let v: Value = serde_json::from_slice(&buf).expect("parses");
        assert_eq!(v["code"], "rewrite-x");
        assert_eq!(v["filename"], "<source>");
        assert_eq!(v["location"], serde_json::json!({"row": 1, "column": 1}));
        assert_eq!(
            v["end_location"],
            serde_json::json!({"row": 1, "column": 2})
        );
        assert_eq!(v["fix"]["applicability"], "safe");
        assert_eq!(v["fix"]["edits"][0]["content"], "y");
    }
}

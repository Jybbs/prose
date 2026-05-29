//! Json emitter: NDJSON of Ruff-shaped diagnostic records closed by a
//! summary envelope.

use std::collections::BTreeMap;
use std::io::{self, Write};

use ruff_diagnostics::{Applicability, Edit};
use ruff_source_file::{LineColumn, OneIndexed, SourceFile};
use ruff_text_size::Ranged;
use serde::Serialize;

use crate::diagnostics::{Diagnostic, Emitter, EmitterSummary, Run, line_columns, write_json_line};
use crate::rule::RuleId;

/// Bumps on any breaking change to existing field shapes, leaving
/// additive fields to land unversioned.
const SCHEMA_VERSION: u32 = 1;

pub(crate) struct Json;

impl Emitter for Json {
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        summary: &EmitterSummary,
    ) -> io::Result<()> {
        for (file, diagnostics) in runs {
            for diag in *diagnostics {
                write_json_line(
                    writer,
                    &JsonRecord::Diagnostic(JsonDiagnostic::new(file, diag)),
                )?;
            }
        }
        write_json_line(writer, &JsonRecord::Summary(JsonSummary::new(summary)))
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
            fix: diag.fix.as_ref().map(|edits| JsonFix::new(file, edits)),
            location: start.into(),
            message: &diag.message,
        }
    }
}

#[derive(Serialize)]
struct JsonEdit<'a> {
    before: &'a str,
    content: &'a str,
    end_location: JsonLocation,
    location: JsonLocation,
}

impl<'a> JsonEdit<'a> {
    fn new(file: &'a SourceFile, edit: &'a Edit) -> Self {
        let (start, end) = line_columns(file, edit.range());
        Self {
            before: &file.source_text()[edit.range()],
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
    fn new(file: &'a SourceFile, edits: &'a [Edit]) -> Self {
        Self {
            applicability: Applicability::Safe,
            edits: edits.iter().map(|edit| JsonEdit::new(file, edit)).collect(),
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

/// One NDJSON line, internally tagged with a leading `kind` discriminator.
#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum JsonRecord<'a> {
    Diagnostic(JsonDiagnostic<'a>),
    Summary(JsonSummary<'a>),
}

#[derive(Serialize)]
struct JsonSummary<'a> {
    diagnostics_total: usize,
    files_changed: usize,
    files_visited: usize,
    prose_version: &'a str,
    rules_fired: &'a BTreeMap<RuleId, usize>,
    schema_version: u32,
}

impl<'a> JsonSummary<'a> {
    fn new(summary: &'a EmitterSummary) -> Self {
        Self {
            diagnostics_total: summary.diagnostics_total,
            files_changed: summary.files_changed,
            files_visited: summary.files_visited,
            prose_version: env!("CARGO_PKG_VERSION"),
            rules_fired: &summary.rules_fired,
            schema_version: SCHEMA_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use ruff_text_size::TextRange;
    use serde_json::{Value, json};

    use super::*;
    use crate::diagnostics::Severity;
    use crate::source::Source;

    fn diag() -> Diagnostic {
        let range = TextRange::new(0.into(), 1.into());
        Diagnostic {
            fix: Some(vec![Edit::range_replacement("y".to_owned(), range)]),
            message: "rewrite x to y".to_owned(),
            range,
            rule: RuleId::from("rewrite-x"),
            severity: Severity::Format,
        }
    }

    fn emit_records(
        source: &Source,
        diagnostics: &[Diagnostic],
        summary: &EmitterSummary,
    ) -> Vec<Value> {
        parse_records(&emit_text(source, diagnostics, summary))
    }

    fn emit_text(source: &Source, diagnostics: &[Diagnostic], summary: &EmitterSummary) -> String {
        let mut buf = Vec::<u8>::new();
        Json.emit(&mut buf, &[(source.source_file(), diagnostics)], summary)
            .expect("emits");
        String::from_utf8(buf).expect("utf-8")
    }

    fn parse_records(text: &str) -> Vec<Value> {
        text.lines()
            .map(|line| serde_json::from_str(line).expect("each line parses as JSON"))
            .collect()
    }

    #[test]
    fn closes_stream_with_a_summary_record_after_each_diagnostic() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let records = emit_records(
            &source,
            std::slice::from_ref(&diag()),
            &EmitterSummary::default(),
        );
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["kind"], "diagnostic");
        assert_eq!(records[1]["kind"], "summary");
    }

    #[test]
    fn edit_before_carries_original_multi_line_substring() {
        let source: Source = "x = 1\ny = 2\n".parse().expect("parses");
        let range = TextRange::new(0.into(), 11.into());
        let diag = Diagnostic {
            fix: Some(vec![Edit::range_replacement("z = 3".to_owned(), range)]),
            message: "collapse".to_owned(),
            range,
            rule: RuleId::from("rewrite-x"),
            severity: Severity::Format,
        };
        let records = emit_records(
            &source,
            std::slice::from_ref(&diag),
            &EmitterSummary::default(),
        );
        assert_eq!(records[0]["fix"]["edits"][0]["before"], "x = 1\ny = 2");
    }

    #[test]
    fn fix_carries_one_edit_entry_per_group_member() {
        let source: Source = "x = 1\ny = 2\n".parse().expect("parses");
        let diag = Diagnostic {
            fix: Some(vec![
                Edit::range_replacement("a".to_owned(), TextRange::new(0.into(), 1.into())),
                Edit::range_replacement("b".to_owned(), TextRange::new(6.into(), 7.into())),
            ]),
            message: "align".to_owned(),
            range: TextRange::new(0.into(), 7.into()),
            rule: RuleId::from("align-equals"),
            severity: Severity::Format,
        };
        let records = emit_records(
            &source,
            std::slice::from_ref(&diag),
            &EmitterSummary::default(),
        );
        let edits = records[0]["fix"]["edits"].as_array().expect("edits array");
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0]["content"], "a");
        assert_eq!(edits[1]["content"], "b");
    }

    #[test]
    fn kind_leads_every_serialized_object() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let text = emit_text(
            &source,
            std::slice::from_ref(&diag()),
            &EmitterSummary::default(),
        );
        let mut lines = text.lines();
        assert!(
            lines
                .next()
                .expect("diagnostic line")
                .starts_with("{\"kind\":\"diagnostic\"")
        );
        assert!(
            lines
                .next()
                .expect("summary line")
                .starts_with("{\"kind\":\"summary\"")
        );
    }

    #[test]
    fn populates_fix_payload_with_safe_applicability_and_edit() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let records = emit_records(
            &source,
            std::slice::from_ref(&diag()),
            &EmitterSummary::default(),
        );
        let diagnostic = &records[0];
        assert_eq!(diagnostic["code"], "rewrite-x");
        assert_eq!(diagnostic["filename"], "<source>");
        assert_eq!(diagnostic["location"], json!({"row": 1, "column": 1}));
        assert_eq!(diagnostic["end_location"], json!({"row": 1, "column": 2}));
        assert_eq!(diagnostic["fix"]["applicability"], "safe");
        assert_eq!(diagnostic["fix"]["edits"][0]["before"], "x");
        assert_eq!(diagnostic["fix"]["edits"][0]["content"], "y");
    }

    #[test]
    fn roundtrips_full_stream_with_deterministic_per_rule_counts() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let range = TextRange::new(0.into(), 1.into());
        let diagnostics = vec![
            diag(),
            Diagnostic::lint(RuleId::from("align-equals"), range, "name it".to_owned()),
        ];
        let rules_fired = BTreeMap::from([
            (RuleId::from("rewrite-x"), 1),
            (RuleId::from("align-equals"), 1),
        ]);
        let summary = EmitterSummary {
            diagnostics_total: 2,
            files_changed: 1,
            files_visited: 1,
            files_with_diagnostics: 1,
            rules_fired,
        };

        let text = emit_text(&source, &diagnostics, &summary);
        assert!(text.contains("\"rules_fired\":{\"align-equals\":1,\"rewrite-x\":1}"));

        let records = parse_records(&text);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0]["kind"], "diagnostic");
        assert_eq!(records[1]["kind"], "diagnostic");
        assert!(records[1]["fix"].is_null());
        assert_eq!(
            records[2],
            json!({
                "kind"              : "summary",
                "diagnostics_total" : 2,
                "files_changed"     : 1,
                "files_visited"     : 1,
                "prose_version"     : env!("CARGO_PKG_VERSION"),
                "rules_fired"       : { "align-equals": 1, "rewrite-x": 1 },
                "schema_version"    : 1,
            }),
        );
    }

    #[test]
    fn summary_closes_zero_diagnostic_stream_with_zero_counts() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let summary = EmitterSummary {
            files_visited: 1,
            ..Default::default()
        };
        let records = emit_records(&source, &[], &summary);
        assert_eq!(records.len(), 1);
        let summary_record = &records[0];
        assert_eq!(summary_record["kind"], "summary");
        assert_eq!(summary_record["diagnostics_total"], 0);
        assert_eq!(summary_record["files_changed"], 0);
        assert_eq!(summary_record["files_visited"], 1);
        assert_eq!(summary_record["rules_fired"], json!({}));
        assert_eq!(summary_record["schema_version"], 1);
        assert_eq!(summary_record["prose_version"], env!("CARGO_PKG_VERSION"));
    }
}

//! Sarif emitter: SARIF 2.1.0 document for GitHub Code Scanning.

use std::io::{self, Write};

use ruff_diagnostics::Edit;
use ruff_source_file::LineColumn;
use ruff_text_size::Ranged;
use serde_sarif::sarif::{
    ArtifactChange, ArtifactContent, ArtifactLocation, Fix, Location, PhysicalLocation, Region,
    Replacement, ReportingDescriptor, Result as SarifResult, ResultLevel, Run as SarifRun,
    Sarif as SarifDoc, ToolComponent,
};

use crate::diagnostics::{Diagnostic, Emitter, Run};
use crate::rule::RuleId;
use crate::source::Source;

pub(crate) struct Sarif;

impl Emitter for Sarif {
    fn emit(&self, writer: &mut dyn Write, runs: &[Run<'_>]) -> io::Result<()> {
        let document = SarifDoc::builder()
            .version("2.1.0")
            .runs(vec![sarif_run(runs)])
            .build();
        serde_json::to_writer(&mut *writer, &document).map_err(io::Error::other)?;
        writer.write_all(b"\n")
    }
}

fn artifact_location(source: &Source) -> ArtifactLocation {
    ArtifactLocation::builder().uri(source.filename()).build()
}

fn collect_rule_ids(runs: &[Run<'_>]) -> Vec<RuleId> {
    let mut ids: Vec<RuleId> = runs
        .iter()
        .flat_map(|(_, diagnostics)| diagnostics.iter().map(|d| d.rule))
        .collect();
    ids.sort_by_key(RuleId::as_str);
    ids.dedup();
    ids
}

fn region(start: LineColumn, end: LineColumn) -> Region {
    Region::builder()
        .start_line(start.line.get() as i64)
        .start_column(start.column.get() as i64)
        .end_line(end.line.get() as i64)
        .end_column(end.column.get() as i64)
        .build()
}

fn sarif_fix(source: &Source, edit: &Edit) -> Fix {
    let start = source.line_column(edit.range().start());
    let end = source.line_column(edit.range().end());
    let inserted = ArtifactContent::builder()
        .text(edit.content().unwrap_or_default())
        .build();
    Fix::builder()
        .artifact_changes(vec![ArtifactChange::builder()
            .artifact_location(artifact_location(source))
            .replacements(vec![Replacement::builder()
                .deleted_region(region(start, end))
                .inserted_content(inserted)
                .build()])
            .build()])
        .build()
}

fn sarif_result(source: &Source, diag: &Diagnostic) -> SarifResult {
    let start = source.line_column(diag.range.start());
    let end = source.line_column(diag.range.end());
    let builder = SarifResult::builder()
        .rule_id(diag.rule.as_str())
        .level(ResultLevel::Warning)
        .message(diag.message.as_str())
        .locations(vec![Location::builder()
            .physical_location(
                PhysicalLocation::builder()
                    .artifact_location(artifact_location(source))
                    .region(region(start, end))
                    .build(),
            )
            .build()]);
    match &diag.fix {
        Some(edit) => builder.fixes(vec![sarif_fix(source, edit)]).build(),
        None => builder.build(),
    }
}

fn sarif_run(runs: &[Run<'_>]) -> SarifRun {
    let rules: Vec<ReportingDescriptor> = collect_rule_ids(runs)
        .iter()
        .map(|id| ReportingDescriptor::builder().id(id.as_str()).build())
        .collect();
    let results: Vec<SarifResult> = runs
        .iter()
        .flat_map(|(source, diagnostics)| {
            diagnostics
                .iter()
                .map(move |diag| sarif_result(source, diag))
        })
        .collect();
    SarifRun::builder()
        .tool(
            ToolComponent::builder()
                .name("prose")
                .version(env!("CARGO_PKG_VERSION"))
                .rules(rules)
                .build(),
        )
        .results(results)
        .build()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;
    use serde_json::Value;

    use super::*;
    use crate::diagnostics::Severity;

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
    fn emits_a_single_sarif_run_per_invocation() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = diag();
        let mut buf = Vec::<u8>::new();
        Sarif
            .emit(&mut buf, &[(&source, std::slice::from_ref(&diag))])
            .expect("emits");
        let v: Value = serde_json::from_slice(&buf).expect("parses as JSON");
        assert_eq!(v["version"], "2.1.0");
        assert_eq!(v["runs"].as_array().expect("runs array").len(), 1);
        let run = &v["runs"][0];
        assert_eq!(run["tool"]["driver"]["name"], "prose");
        assert_eq!(run["tool"]["driver"]["rules"][0]["id"], "rewrite-x");
        let result = &run["results"][0];
        assert_eq!(result["ruleId"], "rewrite-x");
        assert_eq!(result["level"], "warning");
        assert_eq!(result["message"]["text"], "rewrite x to y");
        let region = &result["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 1);
        assert_eq!(region["startColumn"], 1);
        assert_eq!(region["endLine"], 1);
        assert_eq!(region["endColumn"], 2);
    }

    #[test]
    fn populates_fixes_payload_when_diagnostic_carries_an_edit() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = diag();
        let mut buf = Vec::<u8>::new();
        Sarif
            .emit(&mut buf, &[(&source, std::slice::from_ref(&diag))])
            .expect("emits");
        let v: Value = serde_json::from_slice(&buf).expect("parses");
        let fix = &v["runs"][0]["results"][0]["fixes"][0];
        let change = &fix["artifactChanges"][0];
        assert_eq!(change["artifactLocation"]["uri"], "<source>");
        let replacement = &change["replacements"][0];
        assert_eq!(replacement["insertedContent"]["text"], "y");
        let region = &replacement["deletedRegion"];
        assert_eq!(region["startLine"], 1);
        assert_eq!(region["startColumn"], 1);
        assert_eq!(region["endLine"], 1);
        assert_eq!(region["endColumn"], 2);
    }

    #[test]
    fn omits_fixes_payload_when_diagnostic_has_no_edit() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = Diagnostic {
            fix: None,
            ..diag()
        };
        let mut buf = Vec::<u8>::new();
        Sarif
            .emit(&mut buf, &[(&source, std::slice::from_ref(&diag))])
            .expect("emits");
        let v: Value = serde_json::from_slice(&buf).expect("parses");
        assert!(v["runs"][0]["results"][0]["fixes"].is_null());
    }

    #[test]
    fn deduplicates_rule_descriptors_across_diagnostics() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diags = vec![diag(), diag()];
        let mut buf = Vec::<u8>::new();
        Sarif.emit(&mut buf, &[(&source, &diags)]).expect("emits");
        let v: Value = serde_json::from_slice(&buf).expect("parses");
        let rules = v["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules array");
        assert_eq!(rules.len(), 1);
        assert_eq!(
            v["runs"][0]["results"].as_array().expect("results").len(),
            2
        );
    }
}

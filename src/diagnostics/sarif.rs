//! Sarif emitter: SARIF 2.1.0 document for GitHub Code Scanning.

use std::{
    collections::BTreeSet,
    io::{self, Write},
};

use ruff_diagnostics::Edit;
use ruff_source_file::{LineColumn, SourceFile};
use ruff_text_size::Ranged;
use serde_sarif::sarif::{
    ArtifactChange, ArtifactContent, ArtifactLocation, Fix, Location, PhysicalLocation, Region,
    Replacement, ReportingDescriptor, Result as SarifResult, ResultLevel, Run as SarifRun,
    Sarif as SarifDoc, ToolComponent,
};

use crate::{
    diagnostics::{
        Diagnostic, Emitter, EmitterSummary, Run, diagnostics, line_columns, write_json_line,
    },
    file_uri,
    rule::RuleId,
};

pub(crate) struct Sarif;

impl Emitter for Sarif {
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        _summary: &EmitterSummary,
    ) -> io::Result<()> {
        let document = SarifDoc::builder()
            .version("2.1.0")
            .runs(vec![sarif_run(runs)])
            .build();
        write_json_line(writer, &document)
    }
}

fn artifact_location(file: &SourceFile) -> ArtifactLocation {
    ArtifactLocation::builder()
        .uri(file_uri::from_path(file.name()))
        .build()
}

fn collect_rule_ids(runs: &[Run<'_>]) -> Vec<RuleId> {
    diagnostics(runs)
        .map(|(_, d)| d.rule)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn region(start: LineColumn, end: LineColumn) -> Region {
    Region::builder()
        .start_line(start.line.get() as i64)
        .start_column(start.column.get() as i64)
        .end_line(end.line.get() as i64)
        .end_column(end.column.get() as i64)
        .build()
}

fn sarif_fix(file: &SourceFile, edits: &[Edit]) -> Fix {
    let replacements: Vec<Replacement> = edits
        .iter()
        .map(|edit| {
            let (start, end) = line_columns(file, edit.range());
            Replacement::builder()
                .deleted_region(region(start, end))
                .inserted_content(
                    ArtifactContent::builder()
                        .text(edit.content().unwrap_or_default())
                        .build(),
                )
                .build()
        })
        .collect();
    Fix::builder()
        .artifact_changes(vec![
            ArtifactChange::builder()
                .artifact_location(artifact_location(file))
                .replacements(replacements)
                .build(),
        ])
        .build()
}

fn sarif_result(file: &SourceFile, diag: &Diagnostic) -> SarifResult {
    let (start, end) = line_columns(file, diag.range);
    let builder = SarifResult::builder()
        .rule_id(diag.rule.as_str())
        .level(ResultLevel::Warning)
        .message(diag.message.as_str())
        .locations(vec![
            Location::builder()
                .physical_location(
                    PhysicalLocation::builder()
                        .artifact_location(artifact_location(file))
                        .region(region(start, end))
                        .build(),
                )
                .build(),
        ]);
    match &diag.fix {
        Some(fix) => builder.fixes(vec![sarif_fix(file, fix.edits())]).build(),
        None => builder.build(),
    }
}

fn sarif_run(runs: &[Run<'_>]) -> SarifRun {
    let rules: Vec<ReportingDescriptor> = collect_rule_ids(runs)
        .iter()
        .map(|id| ReportingDescriptor::builder().id(id.as_str()).build())
        .collect();
    let results: Vec<SarifResult> = diagnostics(runs)
        .map(|(file, diag)| sarif_result(file, diag))
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
    use ruff_diagnostics::{Edit, Fix};
    use ruff_source_file::SourceFileBuilder;
    use serde_json::Value;

    use super::*;
    use crate::testing::{format_diagnostic, parse, range};

    fn diag() -> Diagnostic {
        format_diagnostic(range(0, 1))
    }

    fn emit_value(file: &SourceFile, diagnostics: &[Diagnostic]) -> Value {
        let mut buf = Vec::<u8>::new();
        Sarif
            .emit(&mut buf, &[(file, diagnostics)], &EmitterSummary::default())
            .expect("emits");
        serde_json::from_slice(&buf).expect("parses")
    }

    #[test]
    fn deduplicates_rule_descriptors_across_diagnostics() {
        let source = parse("x = 1\n");
        let diags = vec![diag(), diag()];
        let v = emit_value(source.source_file(), &diags);
        let rules = v["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules array");
        assert_eq!(rules.len(), 1);
        assert_eq!(
            v["runs"][0]["results"].as_array().expect("results").len(),
            2
        );
    }

    #[test]
    fn emits_a_single_sarif_run_per_invocation() {
        let source = parse("x = 1\n");
        let diag = diag();
        let v = emit_value(source.source_file(), std::slice::from_ref(&diag));
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
    fn encodes_the_absolute_artifact_uri() {
        let file = SourceFileBuilder::new("/tmp/My Project/mod.py", "x = 1\n").finish();
        let v = emit_value(&file, std::slice::from_ref(&diag()));
        let uri = &v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
            ["uri"];
        assert_eq!(uri, "file:///tmp/My%20Project/mod.py");
    }

    #[test]
    fn fix_carries_one_replacement_per_group_edit() {
        let source = parse("x = 1\ny = 2\n");
        let diag = Diagnostic {
            fix: Some(Fix::safe_edits(
                Edit::range_replacement("a".to_owned(), range(0, 1)),
                [Edit::range_replacement("b".to_owned(), range(6, 7))],
            )),
            ..format_diagnostic(range(0, 7))
        };
        let v = emit_value(source.source_file(), std::slice::from_ref(&diag));
        let replacements =
            v["runs"][0]["results"][0]["fixes"][0]["artifactChanges"][0]["replacements"]
                .as_array()
                .expect("replacements array");
        assert_eq!(replacements.len(), 2);
        assert_eq!(replacements[0]["insertedContent"]["text"], "a");
        assert_eq!(replacements[1]["insertedContent"]["text"], "b");
    }

    #[test]
    fn omits_fixes_payload_when_diagnostic_has_no_edit() {
        let source = parse("x = 1\n");
        let diag = Diagnostic {
            fix: None,
            ..diag()
        };
        let v = emit_value(source.source_file(), std::slice::from_ref(&diag));
        assert!(v["runs"][0]["results"][0]["fixes"].is_null());
    }

    #[test]
    fn passes_the_stdin_placeholder_name_through_unchanged() {
        let source = parse("x = 1\n");
        let v = emit_value(source.source_file(), std::slice::from_ref(&diag()));
        let uri = &v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
            ["uri"];
        assert_eq!(uri, "<source>");
    }

    #[test]
    fn populates_fixes_payload_when_diagnostic_carries_an_edit() {
        let source = parse("x = 1\n");
        let diag = diag();
        let v = emit_value(source.source_file(), std::slice::from_ref(&diag));
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
}

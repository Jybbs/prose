//! Per-fixture diagnostic snapshots. `diagnostics.snap` is the
//! `rule@start..end` list `diagnose` collects against the unrewritten
//! source, anchored to the source as written. `lint_findings.snap`
//! renders the lint records against the rewritten output, the buffer the
//! docs site decorates onto its formatted view.

mod common;

use prose::{diagnostics::Diagnostic, source::Source};

fn render(diagnostics: &[Diagnostic]) -> String {
    let mut lines: Vec<String> = diagnostics
        .iter()
        .map(|d| {
            format!(
                "{rule}@{start}..{end}",
                rule = d.rule,
                start = u32::from(d.range.start()),
                end = u32::from(d.range.end()),
            )
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

#[test]
fn fixtures_emit_expected_diagnostics() {
    insta::glob!("fixtures/**/input.py", |path| {
        let domain = common::domain_name(path);
        let (config, harness) = common::fixture_inputs(path);
        let pipeline = common::build_pipeline(domain, &config, &harness);
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");
        let diagnostics = pipeline.diagnose(&source);
        let (output, run_diagnostics) = pipeline.run(source).expect("pipeline runs");

        common::in_snapshot_dir(path, || {
            insta::assert_snapshot!("diagnostics", render(&diagnostics));
            if let Some(json) =
                prose::diagnostics::lint_records_json(output.source_file(), &run_diagnostics)
            {
                insta::assert_snapshot!("lint_findings", json);
            }
        });
    });
}

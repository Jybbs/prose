//! Per-fixture snapshot of the diagnostic list emitted by each rule
//! against its `tests/fixtures/<domain>/<case>/input.py` input,
//! locking in the `rule@start..end` shape against churn.

mod common;

use prose::diagnostics::Diagnostic;
use prose::source::Source;

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
        let (_, diagnostics) = pipeline.run(source).expect("pipeline runs");

        common::in_snapshot_dir(path, || {
            insta::assert_snapshot!("diagnostics", render(&diagnostics));
        });
    });
}

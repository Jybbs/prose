//! Per-fixture snapshot of the diagnostic list emitted by each rule
//! against its `tests/fixtures/<directory>/<case>.input.py` input,
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
    insta::glob!("fixtures/**/*.input.py", |path| {
        let directory = common::directory_name(path);
        let case = common::case_stem(path)
            .strip_suffix(".input")
            .expect("fixture path ends in .input.py");
        let (config, harness) = common::fixture_inputs(path);
        let pipeline = common::build_pipeline(directory, &config, &harness);
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");
        let (_, diagnostics) = pipeline.run(source).expect("pipeline runs");

        common::in_snapshot_dir(directory, || {
            insta::assert_snapshot!(format!("{case}.diagnostics"), render(&diagnostics));
        });
    });
}

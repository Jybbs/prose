//! Per-fixture snapshot of the diagnostic list emitted by each rule
//! against its `tests/fixtures/<directory>/<case>.input.py` input,
//! locking in the `rule@start..end` shape against churn.

mod common;

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;

use prose::config::Config;
use prose::diagnostics::Diagnostic;
use prose::pipeline::Pipeline;
use prose::source::Source;

fn build_pipeline(directory: &str, config: &Config) -> Pipeline {
    match directory {
        "composition" | "suppression" => Pipeline::with_defaults(config),
        "binding_analysis" | "identity" => Pipeline::empty(),
        _ => Pipeline::for_rule(directory, config)
            .unwrap_or_else(|| panic!("no rule registered for fixture directory `{directory}`")),
    }
}

fn fixture_config(path: &Path) -> Config {
    let stem = common::case_stem(path)
        .strip_suffix(".input")
        .expect("fixture path ends in .input.py");
    let sidecar = path.with_file_name(format!("{stem}.config.toml"));
    match fs_err::read_to_string(&sidecar) {
        Ok(c) => {
            Config::from_pyproject_str(&c).unwrap_or_else(|e| panic!("parse sidecar config: {e}"))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Config::default(),
        Err(e) => panic!("read sidecar: {e}"),
    }
}

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
        let directory = path
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .expect("fixture path has a parent directory name");
        let case = common::case_stem(path)
            .strip_suffix(".input")
            .expect("fixture path ends in .input.py");
        let config = fixture_config(path);
        let pipeline = build_pipeline(directory, &config);
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");
        let (_, diagnostics) = pipeline.run(source).expect("pipeline runs");

        common::in_snapshot_dir(directory, || {
            insta::assert_snapshot!(format!("{case}.diagnostics"), render(&diagnostics));
        });
    });
}

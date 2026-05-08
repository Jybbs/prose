//! Per-rule-directory snapshot of the diagnostic list emitted on the
//! canonical (alphabetically-first) fixture, locking in the
//! `rule@start..end` shape against churn.

mod common;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use prose::config::Config;
use prose::diagnostics::Diagnostic;
use prose::pipeline::Pipeline;
use prose::source::Source;

fn build_pipeline(directory: &str, config: &Config) -> Pipeline {
    match directory {
        "composition" | "suppression" => Pipeline::with_defaults(config),
        "identity" => Pipeline::empty(),
        _ => Pipeline::for_rule(directory, config)
            .unwrap_or_else(|| panic!("no rule registered for fixture directory `{directory}`")),
    }
}

/// Returns each `tests/fixtures/<name>/` whose first `.input.py` is
/// the canonical case for that directory. The `config/` directory is
/// skipped because it carries `.toml` cases rather than Python.
fn canonical_fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut canonical: Vec<PathBuf> = fs_err::read_dir(&root)
        .expect("read fixture root")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter_map(|dir| {
            fs_err::read_dir(&dir)
                .expect("read fixture dir")
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(OsStr::to_str)
                        .is_some_and(|s| s.ends_with(".input.py"))
                })
                .min()
        })
        .collect();
    canonical.sort();
    canonical
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
fn rule_directories_emit_expected_diagnostics() {
    for path in canonical_fixtures() {
        let directory = path
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .expect("fixture path has a parent directory name");
        let pipeline = build_pipeline(directory, &Config::default());
        let source = Source::from_path(&path).expect("fixture input reads and parses as Python");
        let (_, diagnostics) = pipeline.run(source).expect("pipeline runs");

        common::in_snapshot_dir(directory, || {
            insta::assert_snapshot!("diagnostics", render(&diagnostics));
        });
    }
}

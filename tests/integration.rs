//! Integration tests exercising each rule against golden-file fixtures.
//!
//! Each rule owns a subdirectory under `tests/fixtures/<rule>/`
//! containing `<case>.input.py` files, each optionally paired with a
//! sibling `<case>.config.toml` carrying a `[tool.prose]` override
//! for that single case. The harness reads the sidecar through
//! `Config::from_pyproject_str` and snapshots the transform output
//! under `tests/snapshots/<rule>/<case>.input.py.snap`, then reruns
//! the pipeline on the formatted text and asserts the second pass
//! produces zero edits and identical bytes.

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use prose::config::Config;
use prose::pipeline::Pipeline;
use prose::source::Source;

fn build_pipeline(rule: &str, config: &Config) -> Pipeline {
    if rule == "identity" {
        return Pipeline::from_rules(Vec::new());
    }
    Pipeline::for_rule(rule, config)
        .unwrap_or_else(|| panic!("no rule registered for fixture directory `{rule}`"))
}

fn fixture_config(input_path: &Path) -> Config {
    let stem = input_path
        .file_name()
        .and_then(OsStr::to_str)
        .and_then(|f| f.strip_suffix(".input.py"))
        .expect("fixture path ends in .input.py");
    let sidecar = input_path.with_file_name(format!("{stem}.config.toml"));
    match fs_err::read_to_string(&sidecar) {
        Ok(contents) => Config::from_pyproject_str(&contents)
            .unwrap_or_else(|e| panic!("parse sidecar {sidecar:?}: {e}")),
        Err(e) if e.kind() == ErrorKind::NotFound => Config::default(),
        Err(e) => panic!("read sidecar {sidecar:?}: {e}"),
    }
}

#[test]
fn fixtures() {
    insta::glob!("fixtures/**/*.input.py", |path| {
        let rule = path
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .expect("fixture path has a parent directory name");
        let case = path
            .file_name()
            .and_then(OsStr::to_str)
            .expect("fixture path has a file name");

        let config = fixture_config(path);
        let pipeline = build_pipeline(rule, &config);
        let source = Source::from_path(path).expect("fixture input reads and parses as Python");

        let (formatted, _) = pipeline
            .run(source)
            .expect("first pass succeeds on fixture input");
        let output = formatted.text().to_owned();

        insta::with_settings!({
            snapshot_path => PathBuf::from("snapshots").join(rule),
            prepend_module_to_snapshot => false,
            snapshot_suffix => "",
        }, {
            insta::assert_snapshot!(case, &output);
        });

        let reparsed = Source::from_str(&output).expect("formatter output reparses as Python");
        let (second, changed) = pipeline.run(reparsed).expect("second pass succeeds");
        assert!(
            !changed,
            "rule `{rule}` not idempotent on `{case}`: second pass emitted edits",
        );
        assert_eq!(
            second.text(),
            output,
            "rule `{rule}` not byte-stable on `{case}`",
        );
    });
}

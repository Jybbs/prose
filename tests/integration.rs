//! Integration tests exercising each rule against golden-file fixtures.
//!
//! Each rule owns a subdirectory under `tests/fixtures/<rule>/`
//! containing one or more `<case>.input.py` files. The expected
//! transform output is an `insta` snapshot under the mirrored path
//! `tests/snapshots/<rule>/<case>.input.py.snap` that `cargo insta
//! review` manages.
//!
//! The `apply` dispatch below maps a fixture directory name to the
//! transform to run. `identity` exists as the seed no-op used to prove
//! the harness works end-to-end; further arms land as each rule issue
//! merges.

use std::ffi::OsStr;
use std::path::PathBuf;

use prose::config::{Config, MaxAlignShiftPolicy};
use prose::pipeline::{Pipeline, Rule};
use prose::rules::align_equals::AlignEquals;
use prose::rules::collection_layout::CollectionLayout;
use prose::source::Source;

fn align_equals_with(policy: MaxAlignShiftPolicy) -> AlignEquals {
    let config = Config {
        max_align_shift_policy: policy,
        ..Config::default()
    };
    AlignEquals::from_config(&config)
}

fn apply(rule: &str, source: Source) -> String {
    let rules: Vec<Box<dyn Rule>> = match rule {
        "align_equals" => vec![Box::new(AlignEquals::default())],
        "align_equals_drop" => vec![Box::new(align_equals_with(MaxAlignShiftPolicy::Drop))],
        "align_equals_skip" => vec![Box::new(align_equals_with(MaxAlignShiftPolicy::Skip))],
        "identity" => Vec::new(),
        "collection_layout" => vec![Box::new(CollectionLayout::default())],
        other => panic!("no transform wired for fixture directory `{other}`"),
    };
    let (formatted, _) = Pipeline::from_rules(rules)
        .run(source)
        .expect("pipeline run succeeds on fixture input");
    formatted.text().to_owned()
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

        let source = Source::from_path(path).expect("fixture input reads and parses as Python");

        insta::with_settings!({
            snapshot_path => PathBuf::from("snapshots").join(rule),
            prepend_module_to_snapshot => false,
            snapshot_suffix => "",
        }, {
            insta::assert_snapshot!(case, apply(rule, source));
        });
    });
}

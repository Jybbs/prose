//! Snapshot tests for the binding-analysis primitive. Each fixture
//! parses, runs `BindingAnalysis::new` through `Source`, and dumps
//! the resulting binding table as YAML for review.

mod common;

use std::ffi::OsStr;

use prose::source::Source;

#[test]
fn binding_tables() {
    insta::glob!("fixtures/binding_analysis/*.input.py", |path| {
        let case = path
            .file_name()
            .and_then(OsStr::to_str)
            .expect("fixture path has a file name");
        let source = Source::from_path(path).expect("fixture parses");
        common::in_snapshot_dir("binding_analysis", || {
            insta::assert_yaml_snapshot!(case, source.binding_analysis());
        });
    });
}

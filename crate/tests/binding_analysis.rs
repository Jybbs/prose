//! Snapshot tests for the binding-analysis primitive. Each fixture
//! parses, runs `BindingAnalysis::new` through `Source`, and dumps
//! the resulting binding table as YAML for review.

mod common;

use prose::source::Source;

#[test]
fn binding_tables() {
    insta::glob!("fixtures/binding_analysis/*/input.py", |path| {
        let source = Source::from_path(path).expect("fixture parses");
        common::in_snapshot_dir(path, || {
            insta::assert_yaml_snapshot!("input.py", source.binding_analysis());
        });
    });
}

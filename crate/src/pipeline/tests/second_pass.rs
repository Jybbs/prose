//! Tests that the narrowed second pass a `format` run makes names every
//! firing rule the full walk names, over each fixture input, so the only
//! rules it can leave out are the ones silent on the first pass.

use std::path::PathBuf;

use super::*;

#[rstest]
fn the_narrowed_second_pass_names_every_firing_rule_the_full_walk_names(
    #[files("../tests/rust/fixtures/**/input.py")]
    #[files("../tests/rust/fixtures/**/input.ipynb")]
    path: PathBuf,
) {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let Ok(source) = Source::from_path(&path) else {
        return;
    };
    let Ok((formatted, _, fired)) = pipeline.run(source) else {
        return;
    };
    let full: Vec<RuleId> = pipeline
        .unsettled(&formatted)
        .into_iter()
        .filter(|rule| fired.contains(rule))
        .collect();
    assert_eq!(pipeline.unsettled_among(&formatted, &fired), full);
}

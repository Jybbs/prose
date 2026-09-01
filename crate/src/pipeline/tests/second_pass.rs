//! Tests that the narrowed second pass a `format` run makes names every
//! firing rule the full walk names, over each corpus input, so the only
//! rules it can leave out are the ones silent on the first pass.

use super::*;
use crate::{diagnostics::fired_rules, testing::corpus_inputs};

#[test]
fn the_narrowed_second_pass_names_every_firing_rule_the_full_walk_names() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    for path in corpus_inputs() {
        let Ok(source) = Source::from_path(&path) else {
            continue;
        };
        let Ok((formatted, diagnostics)) = pipeline.run(source) else {
            continue;
        };
        let fired = fired_rules(&diagnostics);
        let full: Vec<RuleId> = pipeline
            .unsettled(&formatted)
            .into_iter()
            .filter(|rule| fired.contains(rule))
            .collect();
        assert_eq!(
            pipeline.unsettled_among(&formatted, &fired),
            full,
            "{} leaves the narrowed pass short of the full walk over the rules that fired",
            path.display(),
        );
    }
}

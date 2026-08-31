//! Tests that the binding table a reparse carries matches the one a
//! fresh read builds, over each corpus input.

use super::*;
use crate::testing::corpus_inputs;

#[test]
fn carried_binding_tables_match_the_ones_a_fresh_read_builds() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let mut carried = false;
    for path in corpus_inputs() {
        let Ok(mut source) = Source::from_path(&path) else {
            continue;
        };
        let gate = compile_gate(&source, pipeline.target_version);
        for rule in &pipeline.rules {
            source.binding_analysis();
            let Some((_, new_text, map)) = woven_groups(&**rule, &source) else {
                continue;
            };
            let Ok(next) = reparse_or_reject(source, new_text, &**rule, &map, gate) else {
                break;
            };
            source = next;
            carried |= source.assert_carried_bindings_are_fresh(&format!(
                "{} past `{}`",
                path.display(),
                rule.id(),
            ));
        }
    }
    assert!(
        carried,
        "the binding table was never carried across the corpus"
    );
}

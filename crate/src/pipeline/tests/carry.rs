//! Tests that the binding table a reparse carries and the padding walk
//! a splice rebuilds each match the one a fresh read builds, over each
//! corpus input.

use super::*;
use crate::{
    pipeline::error::reparse_or_reject,
    primitives::{edit::apply_edits_mapped, padding::Stranding},
    testing::corpus_inputs,
};

#[test]
fn carried_binding_tables_match_the_ones_a_fresh_read_builds() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let stranding = Stranding::new(RuleId::from("strip-stranded-padding"), true);
    let (mut carried, mut rebuilt) = (false, false);
    for path in corpus_inputs() {
        let Ok(mut source) = Source::from_path(&path) else {
            continue;
        };
        let gate = compile_gate(&source, pipeline.target_version);
        for rule in &pipeline.rules {
            source.binding_analysis();
            source.stranded_padding(stranding);
            let Some(spliceable) = Spliceable::landing(&**rule, &source) else {
                continue;
            };
            let (new_text, map) = apply_edits_mapped(source.text(), spliceable.edits)
                .expect("a landing spliceable weaves");
            let bindings = source.take_binding_analysis();
            let Ok(mut next) = reparse_or_reject(source, new_text, rule.id(), &map, gate) else {
                break;
            };
            next.inherit(bindings, &map, rule.id(), rule.preserves_bindings());
            source = next;
            let site = format!("{} past `{}`", path.display(), rule.id());
            carried |= source.assert_carried_bindings_are_fresh(&site);
            rebuilt |= source.assert_rebuilt_padding_is_fresh(&site);
        }
    }
    assert!(
        carried,
        "the binding table was never carried across the corpus"
    );
    assert!(
        rebuilt,
        "the padding walk was never rebuilt across the corpus"
    );
}

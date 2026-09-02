//! Tests that the binding table a reparse carries and the padding walk
//! a splice rebuilds each match the one a fresh read builds, over each
//! corpus input.

use std::num::NonZeroUsize;

use itertools::Itertools;

use super::*;
use crate::{
    pipeline::error::reparse_or_reject,
    primitives::{edit::apply_edits_mapped, padding::Stranding},
    source::slid_range,
    testing::corpus_inputs,
};

/// The code widths the audit reads every corpus input at.
const WIDTHS: [usize; 6] = [40, 50, 60, 79, 88, 100];

#[test]
fn carried_binding_tables_match_the_ones_a_fresh_read_builds() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let stranding = Stranding::new(RuleId::from("strip-stranded-padding"), true);
    let reservations = Config::default().equals_reservations();
    let (mut carried, mut rebuilt, mut columns) = (false, false, false);
    for path in corpus_inputs() {
        let Ok(mut source) = Source::from_path(&path) else {
            continue;
        };
        let gate = compile_gate(&source, pipeline.target_version);
        for rule in &pipeline.rules {
            source.binding_analysis();
            source.stranded_padding(stranding);
            source.columns(reservations);
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
            columns |= source.assert_carried_columns_are_fresh(&site);
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
    assert!(
        columns,
        "the column table was never rebuilt across the corpus"
    );
}

#[test]
fn column_tables_hold_at_run_granularity_across_every_splice() {
    let mut escapes: Vec<String> = Vec::new();
    let mut splices = 0usize;
    for width in WIDTHS {
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            ..Config::default()
        };
        let pipeline = Pipeline::with_defaults(&config);
        let reservations = config.equals_reservations();
        for path in corpus_inputs() {
            let Ok(mut source) = Source::from_path(&path) else {
                continue;
            };
            let gate = compile_gate(&source, pipeline.target_version);
            for rule in &pipeline.rules {
                let Some(spliceable) = Spliceable::landing(&**rule, &source) else {
                    continue;
                };
                let (new_text, map) = apply_edits_mapped(source.text(), spliceable.edits)
                    .expect("a landing spliceable weaves");
                let Some(splice) = source.splice_of(&new_text, &map) else {
                    let Ok(next) = reparse_or_reject(source, new_text, rule.id(), &map, gate)
                    else {
                        break;
                    };
                    source = next;
                    continue;
                };
                let before = reservations.columns(&source);
                let (held, slid) = (splice.held(), splice.slid());
                let next = source.spliced(new_text, &map, splice, rule.id());
                let after = reservations.columns(&next);
                splices += 1;
                escapes.extend(
                    before
                        .escapes(&after, &map, &held, &slid, |range| slid_range(&map, range))
                        .into_iter()
                        .map(|escape| {
                            format!(
                                "{} past `{}` at width {width}: {escape}",
                                path.display(),
                                rule.id()
                            )
                        }),
                );
                source = next;
            }
        }
    }
    assert!(
        escapes.is_empty(),
        "{} escapes over {splices} splices, e.g.\n{}",
        escapes.len(),
        escapes.iter().take(24).join("\n"),
    );
}

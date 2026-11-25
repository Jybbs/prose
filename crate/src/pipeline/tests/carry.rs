//! Tests that the binding table a reparse carries and the padding walk
//! a splice rebuilds each match the one a fresh read builds, over each
//! fixture input.

use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use itertools::Itertools;

use super::*;
use crate::{
    pipeline::error::reparse_or_reject, primitives::edit::apply_edits_mapped, source::slid_range,
};

/// Whether the binding table was carried, the padding walk rebuilt, and
/// the column table rebuilt past any rule over one input.
#[derive(Debug, Default, PartialEq)]
struct Carried {
    bindings: bool,
    columns: bool,
    padding: bool,
}

#[rstest]
fn carried_binding_tables_match_the_ones_a_fresh_read_builds(
    #[files("../tests/rust/fixtures/**/input.py")]
    #[files("../tests/rust/fixtures/**/input.ipynb")]
    path: PathBuf,
) {
    carried_over(&path);
}

#[test]
fn carrying_reaches_each_table() {
    assert_eq!(
        carried_over(
            &Path::new(prose_harness::FIXTURES)
                .join("align_colons/class_field_run_aligns/input.py")
        ),
        Carried {
            bindings: true,
            columns: true,
            padding: true,
        },
    );
}

#[rstest]
fn column_tables_hold_at_run_granularity_across_every_splice(
    #[files("../tests/rust/fixtures/**/input.py")]
    #[files("../tests/rust/fixtures/**/input.ipynb")]
    path: PathBuf,
    #[values(40, 50, 60, 79, 88, 100)] width: usize,
) {
    let config = Config {
        code_line_length: NonZeroUsize::new(width),
        ..Config::default()
    };
    let pipeline = Pipeline::with_defaults(&config);
    let reservations = config.equals_reservations();
    let Ok(Some(mut source)) = pipeline.read_path(&path) else {
        return;
    };
    let gate = pipeline.gate(&source);
    let mut escapes: Vec<String> = Vec::new();
    for rule in &pipeline.rules {
        let Some(spliceable) = Spliceable::landing(&**rule, &source) else {
            continue;
        };
        let (new_text, map) = apply_edits_mapped(source.text(), spliceable.edits)
            .expect("a landing spliceable weaves");
        let Some(splice) = source.splice_of(&new_text, &map) else {
            let Ok(next) = reparse_or_reject(source, new_text, rule.id(), &map, gate) else {
                break;
            };
            source = next;
            continue;
        };
        let before = reservations.columns(&source);
        let (held, slid) = (splice.held(), splice.slid());
        let next = source.spliced(new_text, &map, splice, rule.id());
        let after = reservations.columns(&next);
        escapes.extend(
            before
                .escapes(&after, &map, &held, &slid, |range| slid_range(&map, range))
                .into_iter()
                .map(|escape| format!("past `{}`: {escape}", rule.id())),
        );
        source = next;
    }
    assert!(escapes.is_empty(), "{}", escapes.iter().join("\n"));
}

/// Runs the default pipeline over `path` one rule at a time, panicking
/// where a carried or rebuilt table differs from a fresh build, and
/// returns which tables it compared.
fn carried_over(path: &Path) -> Carried {
    let config = Config::default();
    let pipeline = Pipeline::with_defaults(&config);
    let stranding = config.stranded_padding();
    let reservations = config.equals_reservations();
    let mut carried = Carried::default();
    let Ok(Some(mut source)) = pipeline.read_path(path) else {
        return carried;
    };
    let gate = pipeline.gate(&source);
    for rule in &pipeline.rules {
        source.binding_analysis();
        source.stranded_padding(stranding);
        source.columns(&reservations);
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
        let site = format!("past `{}`", rule.id());
        carried.bindings |= source.assert_carried_bindings_are_fresh(&site);
        carried.padding |= source.assert_rebuilt_padding_is_fresh(&site);
        carried.columns |= source.assert_carried_columns_are_fresh(&site);
    }
    carried
}

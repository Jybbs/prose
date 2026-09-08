//! Tests for the ratchet a judging run measures its breaks against,
//! covering what a bake writes to the tracked file, which files a read
//! refuses, and which breaks a baseline already carries.

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::{
    ratchet::{
        Baseline, Carried, Counts, VERSION, bake, baseline, baseline_at, dropped, judge,
        regressions,
    },
    records::Width,
    sweep::DEFAULT_LABEL,
};

#[test]
fn a_baked_break_set_reads_back_as_the_set_that_wrote_it() {
    let mut lost = broken("m.py", "re/_parser.py", "leaves `X` unbound");
    lost.names = vec!["X".to_owned()];
    let found = Width {
        breaks: vec![lost],
        candidates: 1,
        comparable: 1,
        label: DEFAULT_LABEL.to_owned(),
        uncomparable: [("blocked.py".to_owned(), "raises".to_owned())].into(),
        ..Width::default()
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let baked = dir.path().join("fresh").join("baseline.json");
    bake(&baked, &[found]);
    let held: Baseline = serde_json::from_str(
        &fs_err::read_to_string(&baked).expect("the baked break set reads back"),
    )
    .expect("the baked break set parses");
    assert_eq!(
        held.breaks[DEFAULT_LABEL],
        [Carried {
            file: "re/_parser.py".to_owned(),
            kind: "unbound".to_owned(),
            module: "m.py".to_owned(),
            names: vec!["X".to_owned()],
        }]
        .into()
    );
    assert_eq!(
        held.uncomparable[DEFAULT_LABEL],
        [("blocked.py".to_owned(), "raises".to_owned())].into()
    );
    assert_eq!(held.version, VERSION);
}

#[test]
fn a_break_set_that_is_older_malformed_or_absent_carries_nothing_forward() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("breaks.json");
    let baked = |version: u32| {
        serde_json::json!({
            "breaks": {
                "default": [{
                    "file": "re/_parser.py",
                    "kind": "unbound",
                    "module": "re",
                    "names": ["X"],
                }],
            },
            "uncomparable": {},
            "version": version,
        })
        .to_string()
    };
    fs_err::write(&path, baked(VERSION)).expect("write the break set");
    assert!(baseline_at(&path).is_some_and(|held| !held.breaks.is_empty()));
    fs_err::write(&path, baked(VERSION - 1)).expect("write the break set");
    assert!(baseline_at(&path).is_none());
    fs_err::write(&path, "{not json").expect("write the break set");
    assert!(baseline_at(&path).is_none());
    assert!(baseline_at(&dir.path().join("absent.json")).is_none());
}

#[test]
fn dropped_names_a_module_the_baseline_does_not_list() {
    let found = Width {
        label: DEFAULT_LABEL.to_owned(),
        uncomparable: [
            ("fresh.py".to_owned(), "raises".to_owned()),
            ("known.py".to_owned(), "raises".to_owned()),
        ]
        .into(),
        ..Width::default()
    };
    let held = Baseline {
        uncomparable: [(
            DEFAULT_LABEL.to_owned(),
            [("known.py".to_owned(), "raises".to_owned())].into(),
        )]
        .into(),
        ..Baseline::default()
    };
    assert_eq!(dropped(&found, &held), ["fresh.py".to_owned()].into());
}

#[test]
fn dropped_names_nothing_where_the_baseline_records_no_width() {
    let found = Width {
        label: DEFAULT_LABEL.to_owned(),
        uncomparable: [("blocked.py".to_owned(), "raises".to_owned())].into(),
        ..Width::default()
    };
    assert_eq!(dropped(&found, &Baseline::default()), BTreeSet::new());
}

#[test]
fn regressions_name_every_count_that_moved_the_wrong_way() {
    let held = Baseline {
        counts: [(
            DEFAULT_LABEL.to_owned(),
            Counts {
                candidates: 997,
                comparable: 898,
                raises: 0,
                rebinds: 0,
                refused: 0,
            },
        )]
        .into(),
        ..Baseline::default()
    };
    let short = Width {
        candidates: 900,
        comparable: 800,
        label: DEFAULT_LABEL.to_owned(),
        refused: 2,
        ..Width::default()
    };
    assert_eq!(
        regressions(&short, &held),
        [
            "candidates 900 against 997 baked",
            "comparable 800 against 898 baked",
            "refused 2 against 0 baked",
        ]
    );
    let reached = Width {
        candidates: 997,
        comparable: 900,
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    assert_eq!(regressions(&reached, &held), Vec::<String>::new());
}

#[test]
fn regressions_name_nothing_where_the_baseline_records_no_counts() {
    let found = Width {
        candidates: 1,
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    assert_eq!(
        regressions(&found, &Baseline::default()),
        Vec::<String>::new()
    );
}

#[test]
fn the_ratchet_carries_a_break_the_baseline_holds_at_the_same_width() {
    let mut lost = broken("m.py", "re/_parser.py", "leaves `X` unbound");
    lost.names = vec!["X".to_owned()];
    let found = Width {
        breaks: vec![lost],
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    let held = Baseline {
        breaks: [(
            DEFAULT_LABEL.to_owned(),
            [Carried {
                file: "re/_parser.py".to_owned(),
                kind: "unbound".to_owned(),
                module: "m.py".to_owned(),
                names: vec!["X".to_owned()],
            }]
            .into(),
        )]
        .into(),
        counts: BTreeMap::new(),
        uncomparable: [(
            DEFAULT_LABEL.to_owned(),
            [("a.py".to_owned(), "raises".to_owned())].into(),
        )]
        .into(),
        version: VERSION,
    };
    assert_eq!(judge(&found, &held), ["m.py".to_owned()].into());
    assert_eq!(judge(&found, &Baseline::default()), BTreeSet::new());
    assert_eq!(
        held.uncomparable[DEFAULT_LABEL],
        [("a.py".to_owned(), "raises".to_owned())].into()
    );
}

#[test]
fn the_tracked_break_set_reads_back_at_the_current_generation() {
    let held = baseline();
    assert_eq!(held.version, VERSION);
    assert!(held.breaks.contains_key(DEFAULT_LABEL));
    assert!(held.uncomparable.contains_key(DEFAULT_LABEL));
}

#[test]
fn two_modules_at_one_frame_bake_as_separate_entries() {
    let found = Width {
        breaks: vec![
            broken("a.py", "compat.py", "raises ImportError: no shutil"),
            broken("b.py", "compat.py", "raises ImportError: no shutil"),
        ],
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let baked = dir.path().join("baseline.json");
    bake(&baked, &[found]);
    let held = baseline_at(&baked).expect("the baked break set reads back");
    assert_eq!(
        held.breaks[DEFAULT_LABEL]
            .iter()
            .map(|entry| entry.module.as_str())
            .collect::<Vec<_>>(),
        ["a.py", "b.py"]
    );
}

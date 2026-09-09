//! Tests for the ratchet a judging run measures its breaks against,
//! covering what a bake writes to the tracked file, which files a read
//! refuses, and which breaks a baseline already carries.

use std::collections::{BTreeMap, BTreeSet};

use rstest::rstest;

use super::*;
use crate::{
    ratchet::{
        Baseline, Carried, Counts, VERSION, bake, baseline, baseline_at, dropped, judge,
        regressions, stale,
    },
    records::{Blocked, Width},
    sweep::DEFAULT_LABEL,
};

/// The entry a baseline holds for `module` losing `name` at `file`.
fn carried(module: &str, file: &str, name: &str) -> Carried {
    Carried {
        file: file.to_owned(),
        kind: "unbound".to_owned(),
        module: module.to_owned(),
        names: vec![name.to_owned()],
    }
}

/// A baseline recording `uncomparable` at the default width and nothing
/// else.
fn recording(uncomparable: BTreeMap<String, Blocked>) -> Baseline {
    Baseline {
        uncomparable: [(DEFAULT_LABEL.to_owned(), uncomparable)].into(),
        ..Baseline::default()
    }
}

/// A width at the default label holding `uncomparable` and nothing else.
fn stalling(uncomparable: BTreeMap<String, Blocked>) -> Width {
    Width {
        uncomparable,
        ..width()
    }
}

#[test]
fn a_baked_break_set_reads_back_as_the_set_that_wrote_it() {
    let found = Width {
        breaks: vec![losing("m.py", "re/_parser.py", "X")],
        candidates: 1,
        comparable: 1,
        flaky: varied([("varies.py", &["N"])]),
        ..stalling(stalled(["blocked.py"]))
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
        [carried("m.py", "re/_parser.py", "X")].into()
    );
    assert_eq!(held.uncomparable[DEFAULT_LABEL], stalled(["blocked.py"]));
    assert_eq!(held.counts[DEFAULT_LABEL].flaky, 1);
    assert_eq!(held.version, VERSION);
}

#[rstest]
#[case::dropped(dropped)]
#[case::judge(judge)]
#[case::stale(stale)]
fn a_baseline_recording_no_width_names_nothing(
    #[case] reading: fn(&Width, &Baseline) -> BTreeSet<String>,
) {
    let found = Width {
        breaks: vec![losing("m.py", "m.py", "X")],
        ..stalling(stalled(["blocked.py"]))
    };
    assert_eq!(reading(&found, &Baseline::default()), BTreeSet::new());
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
    let found = stalling(stalled(["fresh.py", "known.py"]));
    let held = recording(stalled(["known.py"]));
    assert_eq!(dropped(&found, &held), ["fresh.py".to_owned()].into());
}

#[test]
fn dropped_names_nothing_for_a_package_the_machine_lacks() {
    let found = stalling(
        [
            (
                "absent.py".to_owned(),
                blocked(
                    "ModuleNotFoundError",
                    "raises ModuleNotFoundError: No module named 'socks'",
                ),
            ),
            (
                "lost.py".to_owned(),
                blocked("ImportError", "raises ImportError: no thing"),
            ),
        ]
        .into(),
    );
    assert_eq!(
        dropped(&found, &recording(BTreeMap::new())),
        ["lost.py".to_owned()].into()
    );
}

#[test]
fn dropped_reads_the_exception_a_run_named_rather_than_its_sentence() {
    let found = stalling(
        [
            (
                "quoting.py".to_owned(),
                blocked(
                    "ImportError",
                    "raises ImportError: ModuleNotFoundError is not it",
                ),
            ),
            (
                "silent.py".to_owned(),
                blocked("ModuleNotFoundError", "the machine lacks it"),
            ),
        ]
        .into(),
    );
    assert_eq!(
        dropped(&found, &recording(BTreeMap::new())),
        ["quoting.py".to_owned()].into()
    );
}

#[test]
fn regressions_name_every_count_that_moved_the_wrong_way() {
    let held = Baseline {
        counts: [(
            DEFAULT_LABEL.to_owned(),
            Counts {
                candidates: 997,
                comparable: 898,
                flaky: 2,
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
        flaky: varied([("a.py", &[]), ("b.py", &[]), ("c.py", &[])]),
        refused: 2,
        ..width()
    };
    assert_eq!(
        regressions(&short, &held),
        [
            "candidates 900 against 997 baked",
            "comparable 800 against 898 baked",
            "flaky 3 against 2 baked",
            "refused 2 against 0 baked",
        ]
    );
    let reached = Width {
        candidates: 997,
        comparable: 900,
        ..width()
    };
    assert_eq!(regressions(&reached, &held), Vec::<String>::new());
}

#[test]
fn regressions_name_nothing_where_the_baseline_records_no_counts() {
    let found = Width {
        candidates: 1,
        ..width()
    };
    assert_eq!(
        regressions(&found, &Baseline::default()),
        Vec::<String>::new()
    );
}

#[test]
fn stale_names_a_baked_break_the_run_no_longer_reproduces() {
    let held = Baseline {
        breaks: [(
            DEFAULT_LABEL.to_owned(),
            [
                carried("gone.py", "gone.py", "X"),
                carried("kept.py", "kept.py", "Y"),
            ]
            .into(),
        )]
        .into(),
        ..Baseline::default()
    };
    let found = Width {
        breaks: vec![losing("kept.py", "kept.py", "Y")],
        ..stalling(BTreeMap::new())
    };
    assert_eq!(stale(&found, &held), ["gone.py".to_owned()].into());
}

#[test]
fn stale_names_a_break_whose_names_changed_under_one_module() {
    let held = Baseline {
        breaks: [(
            DEFAULT_LABEL.to_owned(),
            [carried("m.py", "m.py", "X")].into(),
        )]
        .into(),
        ..Baseline::default()
    };
    let found = Width {
        breaks: vec![losing("m.py", "m.py", "Y")],
        ..stalling(BTreeMap::new())
    };
    assert_eq!(stale(&found, &held), ["m.py".to_owned()].into());
}

#[test]
fn the_ratchet_carries_a_break_the_baseline_holds_at_the_same_width() {
    let found = Width {
        breaks: vec![losing("m.py", "re/_parser.py", "X")],
        ..stalling(BTreeMap::new())
    };
    let held = Baseline {
        breaks: [(
            DEFAULT_LABEL.to_owned(),
            [carried("m.py", "re/_parser.py", "X")].into(),
        )]
        .into(),
        version: VERSION,
        ..recording(stalled(["a.py"]))
    };
    assert_eq!(judge(&found, &held), ["m.py".to_owned()].into());
    assert_eq!(held.uncomparable[DEFAULT_LABEL], stalled(["a.py"]));
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
        ..width()
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

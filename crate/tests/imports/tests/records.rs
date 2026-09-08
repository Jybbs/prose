//! Tests for the records one sweep leaves behind, covering the modules a
//! break reports as loaded and the counts a width holds.

use std::collections::BTreeSet;

use super::*;
use crate::{
    outcome::{Kind, Outcome},
    records::Width,
    report::render,
    sweep::DEFAULT_LABEL,
};

#[test]
fn a_break_reporting_no_loaded_modules_falls_back_to_its_own() {
    let mut brk = broken("m.py", "m.py", "leaves `X` unbound");
    assert_eq!(brk.loaded(), ["m.py"]);
    brk.formatted.loaded = vec!["a.py".to_owned(), "b.py".to_owned()];
    assert_eq!(brk.loaded(), ["a.py", "b.py"]);
}

#[test]
fn a_carried_break_leaves_the_tally_the_report_renders() {
    let found = Width {
        breaks: vec![
            broken("carried.py", "re/_parser.py", "leaves `X` unbound"),
            broken("fresh.py", "re/_parser.py", "leaves `Y` unbound"),
        ],
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    let carried = ["carried.py".to_owned()].into();
    assert_eq!(
        found
            .uncarried(&carried)
            .map(|brk| brk.module.as_str())
            .collect::<Vec<_>>(),
        ["fresh.py"]
    );
    let shown = render(&carried, &found);
    assert!(shown.contains("  breaks           2"), "{shown}");
    assert!(shown.contains("  carried          1"), "{shown}");
    assert!(shown.contains("fresh.py"), "{shown}");
    assert!(!shown.contains("carried.py"), "{shown}");
}

#[test]
fn a_timing_out_break_counts_as_a_module_rather_than_a_defect() {
    let timed = |module: &str| {
        let mut brk = broken(module, "socket.py", "times out after 30s");
        brk.formatted = Outcome::of(Kind::Timeout, "times out after 30s");
        brk
    };
    let found = Width {
        breaks: vec![timed("a.py"), timed("b.py")],
        candidates: 2,
        comparable: 2,
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    assert_eq!(found.counting(Kind::Timeout), 2);
    let shown = render(&BTreeSet::new(), &found);
    assert!(shown.contains("  timeouts         2"), "{shown}");
    assert!(shown.contains("times out (1):"), "{shown}");
}

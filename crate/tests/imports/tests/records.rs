//! Tests for the records one sweep leaves behind, covering the modules a
//! break reports as loaded and the counts a width holds.

use std::collections::BTreeSet;

use super::*;
use crate::{
    outcome::{Kind, Outcome},
    records::Width,
    report::render,
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
        ..width()
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
    shows(&shown, "  breaks           2");
    shows(&shown, "  carried          1");
    shows(&shown, "fresh.py");
    hides(&shown, "carried.py");
}

#[test]
fn a_flaky_module_counts_its_names_where_one_varying_whole_counts_none() {
    let found = Width {
        flaky: varied([
            ("ctypes/__init__.py", &["_cast_addr", "_string_at_addr"]),
            ("logging/__init__.py", &["_startTime", "raiseExceptions"]),
            ("whole.py", &[]),
        ]),
        ..width()
    };
    assert_eq!(found.flaky.len(), 3);
    assert_eq!(found.varying(), 4);
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
        ..width()
    };
    assert_eq!(found.counting(Kind::Timeout), 2);
    let shown = render(&BTreeSet::new(), &found);
    shows(&shown, "  timeouts         2");
    shows(&shown, "times out (1):");
}

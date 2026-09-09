//! Tests for the rendering of one width's findings, covering the summary
//! block of counts, the reach split beneath it, the listings capped at a
//! shown limit, and the command that reproduces one break alone.

use std::collections::BTreeSet;

use super::*;
use crate::{common::SHOWN, records::Width, report::render};

#[test]
fn a_break_the_report_names_carries_its_frame_reason_and_repro() {
    let mut brk = broken(
        "_colorize.py",
        "re/_parser.py",
        "raises NameError: no MAXGROUPS",
    );
    brk.attribution = "under `prune-inert-imports`".to_owned();
    brk.frame.row = Some(111);
    brk.hunk = vec!["-from _sre import MAXGROUPS".to_owned()];
    let found = Width {
        breaks: vec![brk],
        candidates: 4,
        comparable: 3,
        refused: 1,
        ..width()
    };
    let shown = render(&["kept.py".to_owned()].into(), &found);
    shows(&shown, "  carried          1");
    shows(&shown, "  refused          1");
    hides(&shown, "uncomparable by reach");
    shows(
        &shown,
        "re/_parser.py:111 raises NameError: no MAXGROUPS, under `prune-inert-imports`",
    );
    shows(&shown, "reproduce with mise run imports _colorize.py");
    shows(&shown, "-from _sre import MAXGROUPS");
}

#[test]
fn a_repro_at_a_pinned_width_carries_the_width_knob() {
    let found = Width {
        breaks: vec![losing("pydoc.py", "pydoc.py", "textwrap")],
        label: "100".to_owned(),
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    shows(&shown, "PROSE_SETTLE_WIDTHS=100 mise run imports pydoc.py");
}

#[test]
fn an_unmeasured_module_replaces_the_uncomparable_count() {
    let found = Width {
        uncomparable: stalled(["a.py"]),
        unmeasured: vec!["u.py".to_owned()],
        ..width()
    };
    let shown = render(&BTreeSet::new(), &found);
    shows(&shown, "  uncomparable unmeasured");
    shows(&shown, "unmeasured, a run left no record (1):");
    shows(&shown, "u.py");
}

#[test]
fn the_flaky_list_caps_at_the_shown_limit() {
    let found = Width {
        candidates: SHOWN + 3,
        comparable: SHOWN + 3,
        flaky: (0..SHOWN + 3)
            .map(|n| (format!("m{n:02}.py"), ["N".to_owned()].into()))
            .collect(),
        ..width()
    };
    let shown = render(&BTreeSet::new(), &found);
    shows(
        &shown,
        &format!("flaky, a second run varied ({}):", SHOWN + 3),
    );
    shows(&shown, "... and 3 more");
    hides(&shown, &format!("m{SHOWN}.py"));
}

#[test]
fn the_flaky_listing_names_each_module_beside_the_names_it_varies_on() {
    let found = Width {
        flaky: varied([
            (
                "logging/__init__.py",
                &["_srcfile", "_startTime", "raiseExceptions"],
            ),
            ("whole.py", &[]),
        ]),
        ..width()
    };
    let shown = render(&BTreeSet::new(), &found);
    shows(&shown, "  flaky            2");
    shows(&shown, "  varying          3");
    shows(&shown, "flaky, a second run varied (2):");
    shows(
        &shown,
        "logging/__init__.py  _srcfile, _startTime, raiseExceptions",
    );
    shows(&shown, "whole.py  the whole namespace");
}

#[test]
fn the_reach_split_names_every_class_beneath_the_block() {
    let found = stalling(
        [
            blocked("dbm/gnu.py", "ModuleNotFoundError"),
            blocked("asyncio/windows_events.py", "ImportError"),
            blocked("encodings/mbcs.py", "ImportError"),
        ]
        .into(),
    );
    let shown = render(&BTreeSet::new(), &found);
    shows(&shown, "uncomparable by reach (3):");
    shows(&shown, "1 absent, 1 platform, 1 module");
}

#[test]
fn the_summary_block_holds_every_count_in_one_column() {
    let found = Width {
        candidates: 12,
        comparable: 9,
        uncomparable: stalled(["a.py", "b.py", "c.py"]),
        ..width()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert_eq!(
        shown
            .split("\n\n")
            .next()
            .expect("the block opens the render"),
        concat!(
            "  candidates      12\n",
            "  comparable       9\n",
            "  uncomparable     3\n",
            "  breaks           0\n",
            "  raises           0\n",
            "  rebinds          0\n",
            "  timeouts         0\n",
            "  flaky            0\n",
            "  varying          0\n",
            "  carried          0",
        )
    );
}

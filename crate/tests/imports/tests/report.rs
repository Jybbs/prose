//! Tests for the rendering of one width's findings, covering the summary
//! block of counts, the listings capped at a shown limit, and the command
//! that reproduces one break alone.

use std::collections::BTreeSet;

use super::*;
use crate::{common::SHOWN, records::Width, report::render, sweep::DEFAULT_LABEL};

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
        label: DEFAULT_LABEL.to_owned(),
        refused: 1,
        ..Width::default()
    };
    let shown = render(&["kept.py".to_owned()].into(), &found);
    assert!(shown.contains("  carried          1"), "{shown}");
    assert!(shown.contains("  refused          1"), "{shown}");
    assert!(
        shown.contains(
            "re/_parser.py:111 raises NameError: no MAXGROUPS, under `prune-inert-imports`"
        ),
        "{shown}"
    );
    assert!(
        shown.contains("reproduce with mise run imports _colorize.py"),
        "{shown}"
    );
    assert!(shown.contains("-from _sre import MAXGROUPS"), "{shown}");
}

#[test]
fn a_repro_at_a_pinned_width_carries_the_width_knob() {
    let found = Width {
        breaks: vec![losing("pydoc.py", "pydoc.py", "textwrap")],
        label: "100".to_owned(),
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(
        shown.contains("PROSE_SETTLE_WIDTHS=100 mise run imports pydoc.py"),
        "{shown}"
    );
}

#[test]
fn an_unmeasured_module_replaces_the_uncomparable_count() {
    let found = Width {
        label: DEFAULT_LABEL.to_owned(),
        uncomparable: stalled(["a.py"]),
        unmeasured: vec!["u.py".to_owned()],
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(shown.contains("  uncomparable unmeasured"), "{shown}");
    assert!(
        shown.contains("unmeasured, a run left no record (1):"),
        "{shown}"
    );
    assert!(shown.contains("u.py"), "{shown}");
}

#[test]
fn the_flaky_list_caps_at_the_shown_limit() {
    let found = Width {
        candidates: SHOWN + 3,
        comparable: SHOWN + 3,
        flaky: (0..SHOWN + 3)
            .map(|n| (format!("m{n:02}.py"), ["N".to_owned()].into()))
            .collect(),
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(
        shown.contains(&format!("flaky, a second run varied ({}):", SHOWN + 3)),
        "{shown}"
    );
    assert!(shown.contains("... and 3 more"), "{shown}");
    assert!(!shown.contains(&format!("m{SHOWN}.py")), "{shown}");
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
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(shown.contains("  flaky            2"), "{shown}");
    assert!(shown.contains("  varying          3"), "{shown}");
    assert!(shown.contains("flaky, a second run varied (2):"), "{shown}");
    assert!(
        shown.contains("logging/__init__.py  _srcfile, _startTime, raiseExceptions"),
        "{shown}"
    );
    assert!(shown.contains("whole.py  the whole namespace"), "{shown}");
}

#[test]
fn the_summary_block_holds_every_count_in_one_column() {
    let found = Width {
        candidates: 12,
        comparable: 9,
        label: DEFAULT_LABEL.to_owned(),
        uncomparable: stalled(["a.py", "b.py", "c.py"]),
        ..Width::default()
    };
    assert_eq!(
        render(&BTreeSet::new(), &found),
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

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
        breaks: vec![broken("pydoc.py", "pydoc.py", "leaves `textwrap` unbound")],
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
        uncomparable: vec!["a.py".to_owned()],
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
        flaky: (0..SHOWN + 3).map(|n| format!("m{n}.py")).collect(),
        label: DEFAULT_LABEL.to_owned(),
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(
        shown.contains(&format!(
            "flaky, a second run did not confirm it ({}):",
            SHOWN + 3
        )),
        "{shown}"
    );
    assert!(shown.contains("... and 3 more"), "{shown}");
    assert!(!shown.contains(&format!("m{SHOWN}.py")), "{shown}");
}

#[test]
fn the_summary_block_holds_every_count_in_one_column() {
    let found = Width {
        candidates: 12,
        comparable: 9,
        label: DEFAULT_LABEL.to_owned(),
        uncomparable: vec!["a.py".to_owned(), "b.py".to_owned(), "c.py".to_owned()],
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
            "  flaky            0",
        )
    );
}

#[test]
fn the_summary_names_what_the_baseline_kept_out() {
    let found = Width {
        candidates: 4,
        comparable: 4,
        label: DEFAULT_LABEL.to_owned(),
        skipped: 99,
        ..Width::default()
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(shown.contains("  skipped         99"), "{shown}");
}

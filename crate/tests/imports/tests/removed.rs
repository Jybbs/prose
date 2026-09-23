//! Tests for the names a comparison leaves out, covering a binding any
//! recorded fix removed, a name lost with no fix removing it, and a module
//! whose formatted run never bound a namespace.

use std::path::Path;

use prose::rules::RuleId;
use rstest::rstest;
use ruff_source_file::LineIndex;

use super::*;
use crate::{
    format::edit_rows,
    outcome::{Kind, Outcome},
    records::{EditRows, Fixes, Removed},
    removed::removed,
};

/// The module every case formats, importing `os` and `sys` and reading only
/// `sys`.
const MODULE: &str = "import os\nimport sys\n\nargs = sys.argv\n";

/// What the sweep leaves out of `m.py` where its original bound `was` and
/// its formatted copy left `now`, under `fixes`.
fn left_out(was: Outcome, now: Outcome, fixes: &Fixes) -> Removed {
    let dir = tempfile::tempdir().expect("a scratch directory");
    fs_err::write(dir.path().join("m.py"), MODULE).expect("write the module");
    let module = |ran: Outcome| [("m.py".to_owned(), ran)].into();
    removed(&module(now), &module(was), fixes, dir.path())
}

/// The fixes `rule` records against `m.py` when it removes `import os`.
fn removing_os(rule: &str) -> Fixes {
    let rule: RuleId = rule.parse().expect("the rule is registered");
    let range = 0..10;
    let edit = EditRows {
        content: String::new(),
        rows: edit_rows(&LineIndex::from_source_text(MODULE), MODULE, &range),
        range,
    };
    [("m.py".to_owned(), vec![(rule, vec![edit])])].into()
}

#[rstest]
fn a_binding_any_recorded_fix_removed_leaves_the_comparison(
    #[values("prune-inert-imports", "modernize-annotations")] rule: &str,
) {
    let held = left_out(
        bound(&["args", "os", "sys"], &[]),
        bound(&["args", "sys"], &[]),
        &removing_os(rule),
    );
    assert_eq!(
        held,
        [(
            "m.py".to_owned(),
            [(
                "os".to_owned(),
                format!("`os` bound at m.py:1, dropped by `{rule}`"),
            )]
            .into(),
        )]
        .into()
    );
}

#[test]
fn a_module_absent_from_the_tree_leaves_nothing_out() {
    let fixes = removing_os("prune-inert-imports");
    let module = |names: &[&str]| [("m.py".to_owned(), bound(names, &[]))].into();
    let held = removed(
        &module(&["sys"]),
        &module(&["os", "sys"]),
        &fixes,
        Path::new("/nonexistent"),
    );
    assert_eq!(held, Removed::new());
}

#[test]
fn a_name_no_fix_removed_stays_in_the_comparison() {
    let held = left_out(
        bound(&["args", "os", "sys"], &[]),
        bound(&["os", "sys"], &[]),
        &removing_os("prune-inert-imports"),
    );
    assert_eq!(held, Removed::new());
}

#[rstest]
#[case::the_original(true)]
#[case::the_formatted_copy(false)]
fn a_side_that_raised_leaves_nothing_out(#[case] original_raised: bool) {
    let raised = || Outcome::of(Kind::Raised, "raises NameError: name 'sys' is not defined");
    let (was, now) = if original_raised {
        (raised(), bound(&["args", "sys"], &[]))
    } else {
        (bound(&["args", "os", "sys"], &[]), raised())
    };
    assert_eq!(
        left_out(was, now, &removing_os("prune-inert-imports")),
        Removed::new()
    );
}

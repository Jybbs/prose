//! Tests for which modules of a corpus a sweep runs, covering the entry
//! points the walk leaves out.

use rstest::rstest;

use crate::corpus::{candidates, excluded};

#[test]
fn candidates_drop_the_entry_points_from_the_rewritten_set() {
    let rewritten = ["os.py", "test/x.py", "turtledemo/y.py", "re/_parser.py"]
        .map(str::to_owned)
        .into();
    assert_eq!(candidates(&rewritten), ["os.py", "re/_parser.py"]);
}

#[rstest]
fn entry_points_leave_the_walk(
    #[values(
        "pkg/__main__.py",
        "__main__.py",
        "test/x.py",
        "a/tests/b.py",
        "idlelib/idle_test/x.py",
        "turtledemo/x.py",
        "antigravity.py",
        "idlelib/idle.py",
        "webbrowser.py",
        "config-3.14-darwin/python-config.py",
        "config-3.14-x86_64-linux-gnu/python-config.py"
    )]
    relative: &str,
) {
    assert!(excluded(relative));
}

#[rstest]
fn library_modules_stay_in_the_walk(
    #[values("test_x.py", "unittest/mock.py", "a/testing/b.py", "re/_parser.py")] relative: &str,
) {
    assert!(!excluded(relative));
}

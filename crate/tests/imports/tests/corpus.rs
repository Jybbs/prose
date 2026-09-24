//! Tests for which modules of a corpus a sweep runs, covering the entry
//! points the walk leaves out, the files that name no importable module,
//! the target a version names, and what the run's header names about the
//! corpus.

use rstest::rstest;
use ruff_python_ast::PythonVersion;

use crate::corpus::{candidates, excluded, identity, importable, target, version};

#[rstest]
#[case("3.14.6", PythonVersion::PY314)]
#[case("3.15.0a1", PythonVersion::PY315)]
fn a_target_takes_the_major_and_minor_a_version_names(
    #[case] version: &str,
    #[case] want: PythonVersion,
) {
    assert_eq!(target(version), want);
}

#[test]
#[should_panic(expected = "does not run")]
fn an_interpreter_that_does_not_run_names_itself() {
    version("/nonexistent/python");
}

#[test]
fn candidates_drop_the_entry_points_from_the_read_set() {
    let read = ["os.py", "test/x.py", "turtledemo/y.py", "re/_parser.py"]
        .map(str::to_owned)
        .into();
    assert_eq!(candidates(&read), ["os.py", "re/_parser.py"]);
}

#[test]
fn candidates_drop_the_files_naming_no_importable_module() {
    let read = ["idlelib/idle.pyw", "os.py", "typing.pyi"]
        .map(str::to_owned)
        .into();
    assert_eq!(candidates(&read), ["os.py"]);
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
        "idlelib/idle.pyw",
        "webbrowser.py",
        "config-3.14-darwin/python-config.py",
        "config-3.14-x86_64-linux-gnu/python-config.py"
    )]
    relative: &str,
) {
    assert!(excluded(relative));
}

#[test]
fn identity_counts_every_file_the_widened_walk_reads() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let root = dir.path();
    let write = |name: &str, text: &str| {
        fs_err::write(root.join(name), text).expect("write a corpus file");
    };
    write("mod.py", "x = 1\n");
    write("notes.txt", "left out\n");
    write("book.ipynb", "{}\n");
    let bare = identity(root);
    assert_eq!(bare.files, 1);
    assert_eq!(bare.vendored, Vec::<String>::new());
    write("script.pyw", "y = 2\n");
    write("stub.pyi", "z: int\n");
    assert_eq!(identity(root).files, 3);
    fs_err::create_dir_all(root.join("site-packages/pip-26.2.dist-info")).expect("a dist-info");
    assert_eq!(identity(root).vendored, ["pip-26.2"]);
}

#[rstest]
fn importable_names_only_the_files_an_import_can_bind(
    #[values("os.py", "a/b.py", "pkg/__init__.py")] relative: &str,
) {
    assert!(importable(relative));
}

#[rstest]
fn importable_rejects_the_files_that_name_no_module(
    #[values("idlelib/idle.pyw", "typing.pyi", "notes.txt", "data.ipynb")] relative: &str,
) {
    assert!(!importable(relative));
}

#[rstest]
fn library_modules_stay_in_the_walk(
    #[values("test_x.py", "unittest/mock.py", "a/testing/b.py", "re/_parser.py")] relative: &str,
) {
    assert!(!excluded(relative));
}

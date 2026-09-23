//! Tests for running one module in a fresh interpreter, covering the
//! deadline, the dotted name an import binds it to, how a run that left no
//! record is read, and what the probe records for a staged tree.

use std::{
    assert_matches,
    os::unix::process::ExitStatusExt,
    process::{Command, ExitStatus},
};

use rstest::rstest;
use ruff_python_ast::PythonVersion;

use crate::{
    corpus::{interpreter, target, version},
    execute::{Runner, Waited, ending, module_name, wait},
    outcome::{Kind, Outcome},
};

/// Runs `module` of a scratch corpus holding `files`, each a path beside its
/// text, through the probe every sweep runs, under the resolved interpreter,
/// which has to be 3.14 or later for the annotation cases to test what they
/// name.
fn probed(files: &[(&str, &str)], module: &str) -> Outcome {
    let python = interpreter();
    assert!(
        target(&version(&python)) >= PythonVersion::PY314,
        "the probe tests need the pinned 3.14 interpreter, found {python}",
    );
    let dir = tempfile::tempdir().expect("a scratch corpus");
    for (path, text) in files {
        let file = dir.path().join(path);
        fs_err::create_dir_all(file.parent().expect("a file sits in a directory"))
            .expect("create a corpus directory");
        fs_err::write(file, text).expect("write a corpus file");
    }
    let runner = Runner::new(dir.path(), python);
    runner.run(module, &[&runner.stage.original])
}

#[test]
fn a_clean_exit_without_a_record_is_unmeasured_and_a_dirty_one_raises() {
    assert_eq!(ending(ExitStatus::from_raw(0), "").kind, Kind::Unmeasured);
    let dirty = ending(ExitStatus::from_raw(2 << 8), "boom");
    assert_eq!(dirty.kind, Kind::Raised);
    assert_eq!(dirty.error, "ends on exit status: 2, printing boom");
}

#[rstest]
#[case("os.py", "os")]
#[case("asyncio/queues.py", "asyncio.queues")]
#[case("asyncio/__init__.py", "asyncio")]
#[case("importlib/metadata/__init__.py", "importlib.metadata")]
#[case("site-packages/pip/_internal/cache.py", "pip._internal.cache")]
#[case("site-packages/pip/__init__.py", "pip")]
fn a_module_path_binds_the_name_an_import_binds(#[case] module: &str, #[case] dotted: &str) {
    assert_eq!(module_name(module), dotted);
}

#[test]
fn a_package_that_raises_ends_the_run_naming_the_import() {
    let ran = probed(
        &[
            ("pkg/__init__.py", "import absent_dependency\n"),
            ("pkg/mod.py", "VALUE = 1\n"),
        ],
        "pkg/mod.py",
    );
    assert_eq!(ran.kind, Kind::Raised);
    assert_eq!(ran.importing.as_deref(), Some("absent_dependency"));
}

#[test]
fn a_signal_death_is_a_raise_rather_than_a_timeout() {
    let died = ending(ExitStatus::from_raw(11), "");
    assert_eq!(died.kind, Kind::Raised);
    assert_eq!(died.error, "ends on signal: 11 (SIGSEGV)");
}

#[test]
fn a_vendored_distribution_imports_itself_from_the_tree() {
    let ran = probed(
        &[
            ("site-packages/pkg/__init__.py", "VALUE = 1\n"),
            ("site-packages/pkg/mod.py", "from pkg import VALUE\n"),
        ],
        "site-packages/pkg/mod.py",
    );
    assert_eq!(ran.kind, Kind::Ok);
    assert!(ran.names.contains(&"VALUE".to_owned()), "{:?}", ran.names);
}

#[test]
fn an_annotation_naming_an_unbound_name_raises_when_the_probe_reads_it() {
    let ran = probed(&[("mod.py", "value: Missing = 1\n")], "mod.py");
    assert_eq!(ran.kind, Kind::Raised);
    assert_eq!(ran.raised, "NameError");
}

#[test]
fn the_probe_records_the_names_the_module_annotations_cover() {
    let ran = probed(&[("mod.py", "b: int = 1\na: str\n")], "mod.py");
    assert_eq!(
        ran.constants.get("__annotations__").map(String::as_str),
        Some("('a', 'b')")
    );
}

#[test]
fn wait_kills_a_child_that_outruns_its_deadline() {
    let mut child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("sleep spawns");
    assert_matches!(wait(&mut child, 0.05), Waited::Deadline);
}

#[test]
fn wait_reads_a_child_that_ends_on_its_own() {
    let mut child = Command::new("true").spawn().expect("true spawns");
    assert_matches!(wait(&mut child, 5.0), Waited::Ended(_));
}

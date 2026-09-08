//! Tests for running one module in a fresh interpreter, covering the
//! deadline, the dotted name an import binds it to, and how a run that
//! left no record is read.

use std::{
    assert_matches,
    os::unix::process::ExitStatusExt,
    process::{Command, ExitStatus},
};

use rstest::rstest;

use crate::{
    execute::{Waited, ending, module_name, wait},
    outcome::Kind,
};

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
fn a_module_path_binds_the_name_an_import_binds(#[case] module: &str, #[case] dotted: &str) {
    assert_eq!(module_name(module), dotted);
}

#[test]
fn a_signal_death_is_a_raise_rather_than_a_timeout() {
    let died = ending(ExitStatus::from_raw(11), "");
    assert_eq!(died.kind, Kind::Raised);
    assert_eq!(died.error, "ends on signal: 11 (SIGSEGV)");
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

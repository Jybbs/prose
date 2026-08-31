//! Running one module of a tree in a fresh interpreter, meaning where the
//! module is found, the dotted name an import binds it to, the scratch
//! environment it runs in, and what it left behind.

use std::{
    env,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{records::Outcome, stage::Stage};

/// How often the harness asks whether a run has finished.
const POLL: Duration = Duration::from_millis(20);

/// Runs one module in a fresh interpreter and returns what it left behind.
///
/// The child leads its own process group and carries a scratch `HOME` and
/// `TMPDIR`, so a module that writes or spawns reaches nothing the harness
/// holds. The harness bounds the run rather than the probe, which keeps a
/// module that dies on a signal distinguishable from one the deadline killed.
pub(crate) fn execute(
    stage: &Stage,
    python: &str,
    module: &str,
    trees: &[&Path],
    seconds: f64,
) -> Outcome {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let at = NEXT.fetch_add(1, Ordering::Relaxed);
    let record = stage.records.join(format!("{at}.rec"));
    let printed = stage.records.join(format!("{at}.log"));
    let Some(located) = locate(module, trees) else {
        return Outcome::of("unmeasured", "sits in no tree the run searched".to_owned());
    };
    let Ok(sink) = fs_err::File::create(&printed) else {
        return Outcome::of("unmeasured", "leaves nowhere to print".to_owned());
    };
    let mut command = Command::new(python);
    command
        .arg("-I")
        .arg("-B")
        .arg(stage.probe())
        .arg(&record)
        .arg(module_name(module))
        .arg(located)
        .args(trees)
        .current_dir(&stage.tmp)
        .env_clear()
        .env("HOME", &stage.home)
        .env("PATH", env::var("PATH").unwrap_or_default())
        .env("TMPDIR", &stage.tmp)
        .process_group(0)
        .stderr(Stdio::from(sink.into_parts().0))
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    let Ok(mut child) = command.spawn() else {
        return Outcome::of("unmeasured", "cannot be launched".to_owned());
    };
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    let left = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let Some(status) = left else {
        return Outcome::of("timeout", format!("times out after {seconds}s"));
    };
    if let Ok(record) = fs_err::read_to_string(&record)
        && !record.is_empty()
    {
        return Outcome::parse(&record, trees);
    }
    ending(status, &last_line(&printed))
}

/// What a run that left no record ended as, which is a raise wherever the
/// status is not a clean exit, spelt the way the standard library spells it,
/// so a signal arrives named.
pub(crate) fn ending(status: ExitStatus, printed: &str) -> Outcome {
    if status.code() == Some(0) {
        return Outcome::of("unmeasured", "leaves no record".to_owned());
    }
    let tail = if printed.is_empty() {
        String::new()
    } else {
        format!(", printing {printed}")
    };
    Outcome::of("raised", format!("ends on {status}{tail}"))
}

/// The last line a run printed, or an empty string.
fn last_line(printed: &Path) -> String {
    fs_err::read_to_string(printed)
        .unwrap_or_default()
        .trim()
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// The path of one module under the first of `trees` carrying it.
fn locate(module: &str, trees: &[&Path]) -> Option<PathBuf> {
    trees
        .iter()
        .map(|tree| tree.join(module))
        .find(|path| path.exists())
}

/// The dotted name an import binds one module to.
fn module_name(module: &str) -> String {
    module
        .trim_end_matches(".py")
        .trim_end_matches("/__init__")
        .replace('/', ".")
}

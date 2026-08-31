//! Running one module of a tree in a fresh interpreter, meaning where the
//! module is found, the dotted name an import binds it to, the scratch
//! environment it runs in, and what it left behind.

use std::{
    env,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    common::setting,
    outcome::{Kind, Outcome},
    stage::Stage,
};

/// How often the harness asks whether a run has finished.
const POLL: Duration = Duration::from_millis(20);

/// How many seconds one module may run for absent [`TIMEOUT_VAR`].
const TIMEOUT: f64 = 30.0;

/// The environment variable bounding one module's run, in seconds.
pub(crate) const TIMEOUT_VAR: &str = "PROSE_IMPORTS_TIMEOUT";

/// The interpreter, deadline, and stage every run of a module goes through.
pub(crate) struct Runner {
    /// The interpreter each module runs under.
    python: String,
    /// How many seconds one module may run for.
    seconds: f64,
    /// The scratch stage every copy and run lives in.
    pub(crate) stage: Stage,
}

impl Runner {
    /// Builds the runner, copying the corpus into a fresh stage.
    pub(crate) fn new(corpus: &Path, python: String) -> Self {
        Self {
            python,
            seconds: setting(TIMEOUT_VAR).map_or(TIMEOUT, |held| {
                held.parse()
                    .unwrap_or_else(|_| panic!("`{TIMEOUT_VAR}` is a number of seconds"))
            }),
            stage: Stage::new(corpus),
        }
    }

    /// Runs one module of `trees` in a fresh interpreter and returns what it
    /// left behind.
    ///
    /// The child runs with a scratch `HOME` and `TMPDIR` and leads its own
    /// process group, so it writes nowhere the harness reads and takes no
    /// signal the harness is sent. A timeout kills the child alone, leaving
    /// anything it spawned behind. The harness bounds the run rather than the
    /// probe, which keeps a module that dies on a signal distinguishable from
    /// one the deadline killed.
    pub(crate) fn run(&self, module: &str, trees: &[&Path]) -> Outcome {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let at = NEXT.fetch_add(1, Ordering::Relaxed);
        let record = self.stage.records.join(format!("{at}.rec"));
        let printed = self.stage.records.join(format!("{at}.log"));
        let Some(located) = locate(module, trees) else {
            return Outcome::of(Kind::Unmeasured, "sits in no tree the run searched");
        };
        let Ok(sink) = fs_err::File::create(&printed) else {
            return Outcome::of(Kind::Unmeasured, "leaves nowhere to print");
        };
        let mut command = Command::new(&self.python);
        command
            .arg("-I")
            .arg("-B")
            .arg(self.stage.probe())
            .arg(&record)
            .arg(module_name(module))
            .arg(located)
            .args(trees)
            .current_dir(&self.stage.tmp)
            .env_clear()
            .env("HOME", &self.stage.home)
            .env("PATH", env::var("PATH").unwrap_or_default())
            .env("TMPDIR", &self.stage.tmp)
            .process_group(0)
            .stderr(Stdio::from(sink.into_parts().0))
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        let Ok(mut child) = command.spawn() else {
            return Outcome::of(Kind::Unmeasured, "cannot be launched");
        };
        let Some(status) = wait(&mut child, self.seconds) else {
            return Outcome::of(Kind::Timeout, format!("times out after {}s", self.seconds));
        };
        if let Ok(record) = fs_err::read_to_string(&record)
            && !record.is_empty()
        {
            return Outcome::parse(&record, trees);
        }
        ending(status, &last_line(&printed))
    }
}

/// What a run that left no record ended as, which is a raise wherever the
/// status is not a clean exit, spelt the way the standard library spells it,
/// so a signal arrives named.
pub(crate) fn ending(status: ExitStatus, printed: &str) -> Outcome {
    if status.code() == Some(0) {
        return Outcome::of(Kind::Unmeasured, "leaves no record");
    }
    let tail = if printed.is_empty() {
        String::new()
    } else {
        format!(", printing {printed}")
    };
    Outcome::of(Kind::Raised, format!("ends on {status}{tail}"))
}

/// The dotted name an import binds one module to.
pub(crate) fn module_name(module: &str) -> String {
    module
        .trim_end_matches(".py")
        .trim_end_matches("/__init__")
        .replace('/', ".")
}

/// The last line a run printed, or an empty string.
fn last_line(printed: &Path) -> String {
    fs_err::read_to_string(printed)
        .unwrap_or_default()
        .trim()
        .lines()
        .next_back()
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

/// The status a child ended on, `None` where the deadline killed it first.
fn wait(child: &mut Child, seconds: f64) -> Option<ExitStatus> {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

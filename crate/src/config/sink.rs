//! Prints the notices config resolution raises, one sink taking every
//! notice it is given and the other taking each distinct line once per
//! run.

use std::sync::Mutex;

use rustc_hash::FxHashSet;

use super::notice::ConfigNotice;

/// Prints each distinct notice line once per run, however many config
/// files that run loads. The cwd load and the per-file resolutions share
/// one of these.
#[derive(Default)]
pub(crate) struct NoticeDedup {
    seen: Mutex<FxHashSet<String>>,
}

impl NoticeDedup {
    pub(super) fn emit(&self, notice: ConfigNotice<'_>) {
        let line = notice.to_string();
        if self
            .seen
            .lock()
            .expect("notice dedup lock")
            .insert(line.clone())
        {
            eprintln!("{line}");
        }
    }
}

pub(super) fn emit_notice(notice: ConfigNotice<'_>) {
    eprintln!("{notice}");
}

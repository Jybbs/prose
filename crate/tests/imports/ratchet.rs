//! The ratchet on a width's breaks, meaning the frame set a baseline
//! carries, the modules whose break it already holds, the set a run bakes for
//! the next, and the verdict the run ends on.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    common::setting,
    records::{Break, Width},
};

/// The environment variable naming a file the break set is written to.
const BAKE_VAR: &str = "PROSE_IMPORTS_BAKE";

/// The environment variable naming a break set an earlier run wrote.
const BASELINE_VAR: &str = "PROSE_IMPORTS_BASELINE";

/// The breaks a run left at each frame, keyed by width label.
pub(crate) type Baseline = BTreeMap<String, BTreeSet<Carried>>;

/// The file and reason one break is known by across runs, which is what a
/// baseline carries per break.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Carried {
    /// The file its frame names.
    pub(crate) file: String,
    /// Why the two runs differ.
    pub(crate) reason: String,
}

/// Writes the break set of a run, for a later run to ratchet against.
pub(crate) fn bake(path: &Path, widths: &[Width]) {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent).expect("create the break set's directory");
    }
    let baked: Baseline = widths
        .iter()
        .map(|found| {
            (
                found.label.clone(),
                found.breaks.iter().map(carried).collect(),
            )
        })
        .collect();
    let rendered = serde_json::to_string_pretty(&baked).expect("render the break set");
    fs_err::write(path, rendered + "\n").expect("write the break set");
}

/// The file [`BAKE_VAR`] names, `None` where the variable is unset.
pub(crate) fn baking() -> Option<PathBuf> {
    setting(BAKE_VAR).map(PathBuf::from)
}

/// The break set [`BASELINE_VAR`] names, empty where the variable is unset.
pub(crate) fn baseline() -> Baseline {
    let Some(named) = setting(BASELINE_VAR) else {
        return Baseline::new();
    };
    let held = fs_err::read_to_string(Path::new(&named)).expect("read the baseline");
    serde_json::from_str(&held).expect("parse the baseline")
}

/// The broken modules of one width whose frame file and reason the baseline
/// already holds.
pub(crate) fn judge(found: &Width, held: &Baseline) -> BTreeSet<String> {
    let Some(known) = held.get(&found.label) else {
        return BTreeSet::new();
    };
    found
        .breaks
        .iter()
        .filter(|brk| known.contains(&carried(brk)))
        .map(|brk| brk.module.clone())
        .collect()
}

/// The file and reason a baseline holds one break by.
fn carried(brk: &Break) -> Carried {
    Carried {
        file: brk.frame.file.clone(),
        reason: brk.reason.clone(),
    }
}

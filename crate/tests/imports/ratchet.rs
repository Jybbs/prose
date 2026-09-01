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

/// The environment variable naming a file the armed state is written to.
const ARMED_VAR: &str = "PROSE_IMPORTS_ARMED";

/// The environment variable naming a file the break set is written to.
const BAKE_VAR: &str = "PROSE_IMPORTS_BAKE";

/// The environment variable naming a break set an earlier run wrote.
const BASELINE_VAR: &str = "PROSE_IMPORTS_BASELINE";

/// The generation a baked set is written and read at, raised by every
/// change to what a set carries or to the key one break is held by.
pub(crate) const VERSION: u32 = 1;

/// What one run recorded for a later run to ratchet against, the breaks
/// it left beside the modules it could not compare, each keyed by width
/// label.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Baseline {
    /// The breaks a run left at each frame.
    pub(crate) breaks: BTreeMap<String, BTreeSet<Carried>>,
    /// The modules whose original tree did not run cleanly, which a
    /// later run skips rather than measuring again.
    pub(crate) uncomparable: BTreeMap<String, BTreeSet<String>>,
    /// The generation the set was baked at, `0` for one written before
    /// the field existed.
    pub(crate) version: u32,
}

/// The module, file, and reason one break is known by across runs, which
/// is what a baseline carries per break. The module is part of the key so
/// a fresh module joining a known cascade fails the run rather than
/// matching the entry a sibling already left.
#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(default)]
pub(crate) struct Carried {
    /// The file its frame names.
    pub(crate) file: String,
    /// The module the rewrite broke.
    pub(crate) module: String,
    /// Why the two runs differ.
    pub(crate) reason: String,
}

/// Writes the break set of a run, for a later run to ratchet against.
pub(crate) fn bake(path: &Path, widths: &[Width]) {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent).expect("create the break set's directory");
    }
    let baked = Baseline {
        breaks: widths
            .iter()
            .map(|found| {
                (
                    found.label.clone(),
                    found.breaks.iter().map(carried).collect(),
                )
            })
            .collect(),
        uncomparable: widths
            .iter()
            .map(|found| {
                (
                    found.label.clone(),
                    found.uncomparable.iter().cloned().collect(),
                )
            })
            .collect(),
        version: VERSION,
    };
    let rendered = serde_json::to_string_pretty(&baked).expect("render the break set");
    fs_err::write(path, rendered + "\n").expect("write the break set");
}

/// The file [`BAKE_VAR`] names, `None` where the variable is unset.
pub(crate) fn baking() -> Option<PathBuf> {
    setting(BAKE_VAR).map(PathBuf::from)
}

/// The break set [`BASELINE_VAR`] names, `None` where the variable is
/// unset, no file sits at the path it names, or what sits there was not
/// baked at [`VERSION`].
pub(crate) fn baseline() -> Option<Baseline> {
    let named = setting(BASELINE_VAR)?;
    let held = fs_err::read_to_string(Path::new(&named)).ok()?;
    serde_json::from_str::<Baseline>(&held)
        .ok()
        .filter(|read| read.version == VERSION)
}

/// The broken modules of one width whose module, frame file, and reason
/// the baseline already holds.
pub(crate) fn judge(found: &Width, held: &Baseline) -> BTreeSet<String> {
    let Some(known) = held.breaks.get(&found.label) else {
        return BTreeSet::new();
    };
    found
        .breaks
        .iter()
        .filter(|brk| known.contains(&carried(brk)))
        .map(|brk| brk.module.clone())
        .collect()
}

/// The modules of one width the original tree no longer runs cleanly
/// that the baseline does not already list, meaning the sweep just lost
/// coverage it used to have. A baseline recording nothing at this width
/// carries no coverage to lose, so it names none.
pub(crate) fn dropped(found: &Width, held: &Baseline) -> BTreeSet<String> {
    let Some(known) = held.uncomparable.get(&found.label) else {
        return BTreeSet::new();
    };
    found
        .uncomparable
        .iter()
        .filter(|module| !known.contains(*module))
        .cloned()
        .collect()
}

/// Writes whether the run ratcheted against a baseline to the file
/// [`ARMED_VAR`] names, nothing where the variable is unset.
pub(crate) fn record_armed(armed: bool) {
    if let Some(named) = setting(ARMED_VAR) {
        fs_err::write(Path::new(&named), format!("{armed}\n")).expect("write the armed state");
    }
}

/// The modules a baseline already proved uncomparable at `label`, which
/// a judging run skips rather than paying to measure again.
pub(crate) fn skipping<'a>(held: &'a Baseline, label: &str) -> Option<&'a BTreeSet<String>> {
    held.uncomparable.get(label)
}

/// The module, file, and reason a baseline holds one break by.
fn carried(brk: &Break) -> Carried {
    Carried {
        file: brk.frame.file.clone(),
        module: brk.module.clone(),
        reason: brk.reason.clone(),
    }
}

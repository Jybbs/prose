//! The ratchet on a width's breaks, meaning the frame set a baseline
//! carries, the modules whose break it already holds, the set a run bakes for
//! the next, and the verdict the run ends on.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use crate::{
    common::setting,
    records::{Break, Width},
};

/// The environment variable naming a file the break set is written to.
pub(crate) const BAKE_VAR: &str = "PROSE_IMPORTS_BAKE";

/// The environment variable naming a break set an earlier run wrote.
const BASELINE_VAR: &str = "PROSE_IMPORTS_BASELINE";

/// The file and reason of every frame a run broke at, keyed by width label.
pub(crate) type Baseline = BTreeMap<String, BTreeSet<(String, String)>>;

/// Writes the break set of a run, for a later run to ratchet against.
pub(crate) fn bake(path: &Path, widths: &[Width]) {
    let baked: Baseline = widths
        .iter()
        .map(|found| {
            (
                found.label.clone(),
                found.breaks.iter().map(Break::key).collect(),
            )
        })
        .collect();
    let rendered = serde_json::to_string_pretty(&baked).expect("render the break set");
    fs_err::write(path, rendered + "\n").expect("write the break set");
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
    let known = held.get(&found.label);
    found
        .breaks
        .iter()
        .filter(|brk| known.is_some_and(|known| known.contains(&brk.key())))
        .map(|brk| brk.module.clone())
        .collect()
}

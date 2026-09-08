//! The ratchet a judging run measures its breaks against. It reads the
//! tracked break set, then names the breaks that set already holds and the
//! modules that have fallen out of comparison since it was written. A
//! baking run writes a fresh set instead, into the file
//! `PROSE_IMPORTS_BAKE` names.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    common::setting,
    records::{Break, Width},
};

/// The break set the repository tracks beside the harness, which every
/// judging run reads.
const BAKED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/imports/baseline.json");

/// The environment variable naming a file the break set is written to.
const BAKE_VAR: &str = "PROSE_IMPORTS_BAKE";

/// The generation a baked set is written and read at, raised by every
/// change to what a set carries or to the key that holds one break.
pub(crate) const VERSION: u32 = 4;

/// What one run recorded for a later run to ratchet against, the breaks
/// it left beside the modules it could not compare, each keyed by width
/// label.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Baseline {
    /// The breaks a run left at each frame.
    pub(crate) breaks: BTreeMap<String, BTreeSet<Carried>>,
    /// How many modules each width compared, which a later run must
    /// reach so a corpus that quietly shrinks fails rather than passing.
    pub(crate) floors: BTreeMap<String, Floor>,
    /// The modules whose original tree did not run cleanly, which a
    /// later run skips rather than measuring again.
    pub(crate) uncomparable: BTreeMap<String, BTreeSet<String>>,
    /// The generation the set was baked at, `0` where the file names
    /// none.
    pub(crate) version: u32,
}

/// What a baseline carries per break, being the module, the file its frame
/// names, the kind of difference, and every name it turns on. The module
/// joins the frame in this key, so two modules reaching one frame get
/// separate entries, and the names sit here rather than the rendered
/// sentence so a rewording costs no generation and two different losses
/// cannot share one entry.
#[derive(Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(default)]
pub(crate) struct Carried {
    /// The file its frame names.
    pub(crate) file: String,
    /// What kind of difference this is.
    pub(crate) kind: String,
    /// The module the rewrite broke.
    pub(crate) module: String,
    /// Every name the difference turns on, sorted.
    pub(crate) names: Vec<String>,
}

/// The counts one width reached, which a later run compares against so a
/// corpus that shrinks fails rather than passing on less work.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Floor {
    /// How many modules the sweep was eligible to compare.
    pub(crate) candidates: usize,
    /// How many of those the original tree ran cleanly.
    pub(crate) comparable: usize,
    /// How many modules the format run could not read, parse, or write.
    pub(crate) refused: usize,
}

/// Writes the break set of a run, for a later run to ratchet against.
pub(crate) fn bake(path: &Path, widths: &[Width]) {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent).expect("create the break set's directory");
    }
    let baked = Baseline {
        breaks: keyed(widths, |found| found.breaks.iter().map(carried).collect()),
        floors: widths
            .iter()
            .map(|found| {
                (
                    found.label.clone(),
                    Floor {
                        candidates: found.candidates,
                        comparable: found.comparable,
                        refused: found.refused,
                    },
                )
            })
            .collect(),
        uncomparable: keyed(widths, |found| found.uncomparable.iter().cloned().collect()),
        version: VERSION,
    };
    let rendered = serde_json::to_string_pretty(&baked).expect("render the break set");
    fs_err::write(path, rendered + "\n").expect("write the break set");
}

/// The file [`BAKE_VAR`] names, `None` where the variable is unset.
pub(crate) fn baking() -> Option<PathBuf> {
    setting(BAKE_VAR).map(PathBuf::from)
}

/// The tracked break set at [`BAKED`], which every judging run ratchets
/// against. Panics where nothing readable sits at that path, and where the
/// set it finds was baked at another generation.
pub(crate) fn baseline() -> Baseline {
    baseline_at(Path::new(BAKED)).unwrap_or_else(|| {
        panic!(
            "{BAKED} holds no break set baked at generation {VERSION}, so re-bake it with \
             PROSE_IMPORTS_BAKE={BAKED} mise run imports",
        )
    })
}

/// The break set at `path`, `None` where it is unreadable, malformed,
/// or baked at another generation.
pub(crate) fn baseline_at(path: &Path) -> Option<Baseline> {
    let held = fs_err::read_to_string(path).ok()?;
    serde_json::from_str::<Baseline>(&held)
        .ok()
        .filter(|read| read.version == VERSION)
}

/// The modules of one width that the original tree no longer runs cleanly
/// and the baseline does not already list, meaning coverage the sweep just
/// lost. A baseline recording nothing at this width has no coverage to
/// lose, so it names none.
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

/// The broken modules of one width the baseline already holds, matched on
/// the module, the file its frame names, and the reason.
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

/// How one width fell short of the counts the baseline recorded, empty
/// where it reached every one. A baseline holding no floor at this width
/// records nothing to fall short of.
pub(crate) fn shortfalls(found: &Width, held: &Baseline) -> Vec<String> {
    let Some(floor) = held.floors.get(&found.label) else {
        return Vec::new();
    };
    [
        ("candidates", found.candidates, floor.candidates),
        ("comparable", found.comparable, floor.comparable),
    ]
    .into_iter()
    .filter(|(_, reached, baked)| reached < baked)
    .map(|(what, reached, baked)| format!("{what} {reached} against {baked} baked"))
    .chain(
        (found.refused > floor.refused)
            .then(|| format!("refused {} against {} baked", found.refused, floor.refused)),
    )
    .collect()
}

/// The module, file, kind, and names a baseline holds one break by.
fn carried(brk: &Break) -> Carried {
    Carried {
        file: brk.frame.file.clone(),
        kind: brk.kind.to_owned(),
        module: brk.module.clone(),
        names: brk.names.clone(),
    }
}

/// The set each width projects, keyed by that width's label.
fn keyed<T: Ord>(
    widths: &[Width],
    of: impl Fn(&Width) -> BTreeSet<T>,
) -> BTreeMap<String, BTreeSet<T>> {
    widths
        .iter()
        .map(|found| (found.label.clone(), of(found)))
        .collect()
}

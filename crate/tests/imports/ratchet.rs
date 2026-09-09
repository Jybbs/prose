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
    outcome::Kind,
    records::{Blocked, Break, Corpus, Width},
};

/// The break set the repository tracks beside the harness, which every
/// judging run reads.
const BAKED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/imports/baseline.json");

/// The environment variable naming a file the break set is written to.
const BAKE_VAR: &str = "PROSE_IMPORTS_BAKE";

/// The generation a baked set is written and read at, raised by every
/// change to what a set carries or to the key that holds one break.
pub(crate) const VERSION: u32 = 7;

/// What one run recorded for a later run to ratchet against, the breaks
/// it left beside the modules it could not compare, each keyed by width
/// label.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Baseline {
    /// The breaks a run left at each frame.
    pub(crate) breaks: BTreeMap<String, BTreeSet<Carried>>,
    /// The corpus the run swept, which a later run compares its own against,
    /// so a corpus that changed is reported as that rather than as counts
    /// that no longer add up.
    pub(crate) corpus: Corpus,
    /// What each width counted, which a later run measures itself
    /// against so a corpus that shrinks or a defect class that grows
    /// fails rather than passing.
    pub(crate) counts: BTreeMap<String, Counts>,
    /// The modules whose original tree did not run cleanly, each beside
    /// what its run left. A later run keys on the module alone, so an
    /// interpreter rewording an error churns no entry.
    pub(crate) uncomparable: BTreeMap<String, BTreeMap<String, Blocked>>,
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

/// What one width counted. The first two are floors a later run must
/// reach, so a shrinking corpus fails, and the rest are ceilings it must
/// not exceed, so a growing defect class fails. Splitting a raise from a
/// rebind keeps a module that fails to import apart from one that ran and
/// bound a different namespace, since only the first is broken.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Counts {
    /// How many modules the sweep was eligible to compare.
    pub(crate) candidates: usize,
    /// How many of those the original tree ran cleanly.
    pub(crate) comparable: usize,
    /// How many modules failed to import at all.
    pub(crate) raises: usize,
    /// How many ran and bound a different namespace.
    pub(crate) rebinds: usize,
    /// How many modules the format run could not read, parse, or write.
    pub(crate) refused: usize,
}

impl From<&Width> for Counts {
    fn from(found: &Width) -> Self {
        Self {
            candidates: found.candidates,
            comparable: found.comparable,
            raises: found.unimported(),
            rebinds: found.counting(Kind::Ok),
            refused: found.refused,
        }
    }
}

/// Writes the break set of a run, for a later run to ratchet against.
pub(crate) fn bake(path: &Path, corpus: &Corpus, widths: &[Width]) {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent).expect("create the break set's directory");
    }
    let baked = Baseline {
        breaks: keyed(widths, |found| found.breaks.iter().map(carried).collect()),
        corpus: corpus.clone(),
        counts: widths
            .iter()
            .map(|found| (found.label.clone(), Counts::from(found)))
            .collect(),
        uncomparable: widths
            .iter()
            .map(|found| (found.label.clone(), found.uncomparable.clone()))
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
/// lost. A module this machine cannot reach is left out, because a machine
/// lacking a package it imports or running another platform drops that module
/// on every run, and the `comparable` floor is what covers it instead. A
/// baseline recording nothing at this width has no coverage to lose, so it
/// names none.
pub(crate) fn dropped(found: &Width, held: &Baseline) -> BTreeSet<String> {
    let Some(known) = held.uncomparable.get(&found.label) else {
        return BTreeSet::new();
    };
    found
        .uncomparable
        .iter()
        .filter(|(module, left)| !known.contains_key(*module) && left.reach.reachable())
        .map(|(module, _)| module.clone())
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

/// The breaks the baseline holds at one width that this run did not
/// reproduce, meaning the tracked set now overstates what the tree breaks
/// and a re-bake is owed. A run cannot tell a break someone fixed from one
/// a narrowed sweep stopped reaching, so both are reported and both are
/// settled by re-baking.
pub(crate) fn stale(found: &Width, held: &Baseline) -> BTreeSet<String> {
    let Some(known) = held.breaks.get(&found.label) else {
        return BTreeSet::new();
    };
    let reproduced: BTreeSet<_> = found.breaks.iter().map(carried).collect();
    known
        .difference(&reproduced)
        .map(|entry| entry.module.clone())
        .collect()
}

/// How the corpus this run swept differs from the one the baseline
/// records, `None` where the two match. A baseline that records no corpus has
/// nothing to differ from, so a set baked before the corpus was tracked reads
/// as unmoved.
pub(crate) fn moved(swept: &Corpus, held: &Baseline) -> Option<String> {
    let baked = &held.corpus;
    if *baked == Corpus::default() || baked == swept {
        return None;
    }
    Some(format!(
        "interpreter {} against {} baked, {} files against {}, digest {} against {}",
        swept.interpreter, baked.interpreter, swept.files, baked.files, swept.digest, baked.digest,
    ))
}

/// How one width moved the wrong way against the counts the baseline
/// recorded, empty where every one held. A baseline recording nothing at
/// this width has nothing to move against.
pub(crate) fn regressions(found: &Width, held: &Baseline) -> Vec<String> {
    let Some(baked) = held.counts.get(&found.label) else {
        return Vec::new();
    };
    let reached = Counts::from(found);
    let short = [
        ("candidates", reached.candidates, baked.candidates),
        ("comparable", reached.comparable, baked.comparable),
    ]
    .into_iter()
    .filter(|(_, reached, want)| reached < want);
    let grown = [
        ("raises", reached.raises, baked.raises),
        ("rebinds", reached.rebinds, baked.rebinds),
        ("refused", reached.refused, baked.refused),
    ]
    .into_iter()
    .filter(|(_, reached, want)| reached > want);
    short
        .chain(grown)
        .map(|(what, reached, want)| format!("{what} {reached} against {want} baked"))
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

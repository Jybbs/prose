//! The records one sweep leaves, meaning a module the rewrite breaks, the
//! frame it points at, one edit of a recorded fix, why a module could not be
//! compared, one width's tallies and findings, and the corpus the run read.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display, Formatter},
    ops::Range,
};

use prose::rules::RuleId;
use serde::{Deserialize, Serialize};

use crate::outcome::{Kind, Outcome};

/// The exception a module raises where the machine lacks a package it
/// imports.
const ABSENT: &str = "ModuleNotFoundError";

/// The platform tokens that appear in the path of a module written for another
/// operating system. Such a module's own error message often does not name the
/// platform, so the path is what identifies it.
const PLATFORMS: &[&str] = &["darwin", "emscripten", "macos", "win32", "windows"];

/// What one uncomparable module's own run left, the exception it named
/// beside the sentence a report shows, so a later read matches the
/// exception rather than searching the sentence for it, and the reach saying
/// how far this machine gets with the module.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Blocked {
    /// The exception the run named, empty where it named none.
    pub(crate) raised: String,
    /// How far a run on this machine reaches the module.
    pub(crate) reach: Reach,
    /// The sentence a report shows.
    pub(crate) reason: String,
}

impl Blocked {
    /// What blocked the module at `relative`, whose run raised `raised`
    /// and reads as `reason`.
    pub(crate) fn of(relative: &str, raised: &str, reason: &str) -> Self {
        Self {
            raised: raised.to_owned(),
            reach: Reach::of(relative, raised),
            reason: reason.to_owned(),
        }
    }
}

/// A module the rewrite breaks.
pub(crate) struct Break {
    /// The rules and binding the run traced it to.
    pub(crate) attribution: String,
    /// What the run from the formatted tree left behind.
    pub(crate) formatted: Outcome,
    /// The file and row it points at.
    pub(crate) frame: Frame,
    /// The diff lines around that row.
    pub(crate) hunk: Vec<String>,
    /// What kind of difference this is, stable under any rewording.
    pub(crate) kind: &'static str,
    /// The module, relative to its tree.
    pub(crate) module: String,
    /// The name it turns on, where it has one.
    pub(crate) name: Option<String>,
    /// Every name the difference turns on, sorted.
    pub(crate) names: Vec<String>,
    /// What the run from the original tree left behind.
    pub(crate) original: Outcome,
    /// Why the two runs differ, as a sentence predicate.
    pub(crate) reason: String,
}

impl Break {
    /// The modules the formatted run loaded from its tree, or the module
    /// itself where the run recorded none.
    pub(crate) fn loaded(&self) -> Vec<String> {
        if self.formatted.loaded.is_empty() {
            vec![self.module.clone()]
        } else {
            self.formatted.loaded.clone()
        }
    }
}

/// What one run swept, naming the interpreter that owns the corpus beside the
/// distributions installed next to it, so a corpus that changed between two
/// runs is reported as that rather than as counts that no longer add up.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Corpus {
    /// How many files the walk read.
    pub(crate) files: usize,
    /// The version of the interpreter the corpus belongs to.
    pub(crate) interpreter: String,
    /// Every distribution installed beside the standard library, each as
    /// the name and version its `dist-info` directory carries.
    pub(crate) vendored: Vec<String>,
}

/// One edit of a recorded fix, as the span it rewrote and the text it wrote.
pub(crate) struct EditRows {
    /// The text the edit wrote.
    pub(crate) content: String,
    /// The byte range it rewrote.
    pub(crate) range: Range<usize>,
    /// The original rows it rewrote.
    pub(crate) rows: Range<usize>,
}

/// The safe fixes one format run recorded, keyed by the file each rewrote.
pub(crate) type Fixes = BTreeMap<String, Vec<(RuleId, Vec<EditRows>)>>;

/// The file and row a break points at.
#[derive(Clone, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Frame {
    /// The file, relative to the tree it was found in.
    pub(crate) file: String,
    /// The row it names, where the traceback gave one.
    pub(crate) row: Option<usize>,
}

/// How far a run on this machine gets with a module the sweep could not
/// compare. It separates the modules nothing here could import from the ones
/// that fail on their own terms.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Reach {
    /// The module imports a package this machine does not carry.
    Absent,
    /// The module runs here and fails on its own terms.
    #[default]
    Module,
    /// The module is written for another platform.
    Platform,
}

impl Reach {
    /// The reach of the module at `relative`, whose run raised `raised`. The
    /// exception is read first and the path second, since a module written for
    /// another platform often raises an error that does not name one.
    pub(crate) fn of(relative: &str, raised: &str) -> Self {
        if raised == ABSENT {
            Self::Absent
        } else if PLATFORMS.iter().any(|token| relative.contains(token)) {
            Self::Platform
        } else {
            Self::Module
        }
    }

    /// Reports whether a run here reaches the module at all. One naming an
    /// absent package or another platform does not.
    pub(crate) const fn reachable(self) -> bool {
        matches!(self, Self::Module)
    }
}

impl Display for Reach {
    fn fmt(&self, form: &mut Formatter<'_>) -> fmt::Result {
        form.write_str(match self {
            Self::Absent => "absent",
            Self::Module => "module",
            Self::Platform => "platform",
        })
    }
}

/// One width's tallies and findings.
#[derive(Default)]
pub(crate) struct Width {
    /// Every module the rewrite breaks at this width.
    pub(crate) breaks: Vec<Break>,
    /// How many modules the sweep was eligible to compare.
    pub(crate) candidates: usize,
    /// How many of those the original tree ran cleanly.
    pub(crate) comparable: usize,
    /// The modules whose two runs of the original differed, each beside the
    /// names it varied on. An empty set stands for a module whose entire
    /// namespace varied, which a comparison sets aside rather than
    /// narrowing.
    pub(crate) flaky: BTreeMap<String, BTreeSet<String>>,
    /// The width, or `default` where none was pinned.
    pub(crate) label: String,
    /// How many modules the format run could not read, parse, or write.
    pub(crate) refused: usize,
    /// The modules the original tree did not run cleanly, each beside
    /// what its run left, which a run therefore never judges.
    pub(crate) uncomparable: BTreeMap<String, Blocked>,
    /// The modules a run left no record for.
    pub(crate) unmeasured: Vec<String>,
}

impl Width {
    /// How many of this width's breaks left a run of `kind`, which
    /// separates a module that raised from one the deadline killed and
    /// from one that ran and bound a different namespace.
    pub(crate) fn counting(&self, kind: Kind) -> usize {
        self.breaks
            .iter()
            .filter(|brk| brk.formatted.kind == kind)
            .count()
    }

    /// How this width's uncomparable modules divide by reach, each class
    /// paired with its count, in the order a report names them.
    pub(crate) fn reaches(&self) -> [(Reach, usize); 3] {
        [Reach::Absent, Reach::Platform, Reach::Module].map(|reach| {
            let counted = self
                .uncomparable
                .values()
                .filter(|left| left.reach == reach)
                .count();
            (reach, counted)
        })
    }

    /// How many modules this machine could reach, being the ones it compared
    /// beside the ones no run here could import. A module bound to another
    /// platform or naming an absent package holds this count where it moves
    /// `comparable`, so the floor reads the same on every machine.
    pub(crate) fn reachable(&self) -> usize {
        let unreachable = self
            .uncomparable
            .values()
            .filter(|left| !left.reach.reachable())
            .count();
        self.comparable + unreachable
    }

    /// The breaks at this width the baseline does not already hold.
    pub(crate) fn uncarried<'a>(
        &'a self,
        carried: &'a BTreeSet<String>,
    ) -> impl Iterator<Item = &'a Break> {
        self.breaks
            .iter()
            .filter(move |brk| !carried.contains(&brk.module))
    }

    /// How many of this width's breaks never imported at all, counting a
    /// module that raised beside one the deadline killed.
    pub(crate) fn unimported(&self) -> usize {
        self.breaks
            .iter()
            .filter(|brk| brk.formatted.kind != Kind::Ok)
            .count()
    }

    /// How many names this width set aside across its flaky modules,
    /// counting a name once per module that varies on it.
    pub(crate) fn varying(&self) -> usize {
        self.flaky.values().map(BTreeSet::len).sum()
    }
}

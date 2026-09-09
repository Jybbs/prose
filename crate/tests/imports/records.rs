//! The records one sweep leaves, meaning a module the rewrite breaks, the
//! frame it points at, one edit of a recorded fix, and one width's tallies
//! and findings.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use prose::rules::RuleId;
use serde::{Deserialize, Serialize};

use crate::outcome::{Kind, Outcome};

/// What one uncomparable module's own run left, the exception it named
/// beside the sentence a report shows, so a later read matches the
/// exception rather than searching the sentence for it.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Blocked {
    /// The exception the run named, empty where it named none.
    pub(crate) raised: String,
    /// The sentence a report shows.
    pub(crate) reason: String,
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

//! The records one sweep leaves, meaning a module the rewrite breaks, the
//! frame it points at, one edit of a recorded fix, why a module could not be
//! compared, the names a comparison left out, one width's tallies and
//! findings, and the corpus the run read.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use prose::rules::RuleId;

use crate::{
    outcome::{Kind, Outcome},
    reach::Reach,
};

/// What one uncomparable module's own run left, the sentence a report shows
/// beside the reach saying how far this machine gets with the module.
pub(crate) struct Blocked {
    /// How far a run on this machine reaches the module.
    pub(crate) reach: Reach,
    /// The sentence a report shows.
    pub(crate) reason: String,
}

impl Blocked {
    /// What blocked the module at `relative`, whose run left `ran`.
    pub(crate) fn of(relative: &str, ran: &Outcome) -> Self {
        Self {
            reach: Reach::of(relative, ran),
            reason: ran.error.clone(),
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
    /// The module, relative to its tree.
    pub(crate) module: String,
    /// The name it turns on, where it has one.
    pub(crate) name: Option<String>,
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

/// What one run swept, the files the walk read beside the distributions
/// installed next to the standard library.
pub(crate) struct Corpus {
    /// How many files the walk read.
    pub(crate) files: usize,
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

/// The names the comparison left out of each module, each beside the clause
/// naming where the original bound it and the rules whose fixes removed it,
/// keyed by module.
pub(crate) type Removed = BTreeMap<String, BTreeMap<String, String>>;

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
    /// Each file the pipeline read and could not format, beside the error it
    /// returned, every one counting as a break.
    pub(crate) rejected: BTreeMap<String, String>,
    /// The names the comparison left out, since a recorded fix removed the
    /// binding of each.
    pub(crate) removed: Removed,
    /// How many files the format run rewrote.
    pub(crate) rewritten: usize,
    /// The modules the original tree did not run cleanly, each beside
    /// what its run left, which a run therefore never judges.
    pub(crate) uncomparable: BTreeMap<String, Blocked>,
    /// The modules a run left no record for.
    pub(crate) unmeasured: Vec<String>,
    /// How many files the pipeline could not read or parse, which the run
    /// leaves out.
    pub(crate) unread: usize,
}

impl Width {
    /// How many breaks this width found, counting a file the pipeline could
    /// not format beside a module the rewrite broke.
    pub(crate) fn broken(&self) -> usize {
        self.breaks.len() + self.rejected.len()
    }

    /// How many of this width's breaks left a run of `kind`, which
    /// separates a module that raised from one the deadline killed and
    /// from one that ran and bound a different namespace.
    pub(crate) fn counting(&self, kind: Kind) -> usize {
        self.breaks
            .iter()
            .filter(|brk| brk.formatted.kind == kind)
            .count()
    }

    /// How many names the comparison left out across every module.
    pub(crate) fn left_out(&self) -> usize {
        self.removed.values().map(BTreeMap::len).sum()
    }

    /// How this width's uncomparable modules divide by reach, each class
    /// paired with its count, in the order a report names them.
    pub(crate) fn reaches(&self) -> [(Reach, usize); 4] {
        [Reach::Absent, Reach::Platform, Reach::Module, Reach::Loader].map(|reach| {
            let counted = self
                .uncomparable
                .values()
                .filter(|left| left.reach == reach)
                .count();
            (reach, counted)
        })
    }

    /// How many names this width set aside across its flaky modules,
    /// counting a name once per module that varies on it.
    pub(crate) fn varying(&self) -> usize {
        self.flaky.values().map(BTreeSet::len).sum()
    }
}

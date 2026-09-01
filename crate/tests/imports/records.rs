//! The records one sweep leaves, meaning a module the rewrite breaks, the
//! frame it points at, one edit of a recorded fix, and one width's tallies
//! and findings.

use std::{collections::BTreeMap, ops::Range};

use prose::rule::RuleId;

use crate::outcome::{Kind, Outcome};

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
pub(crate) struct Width {
    /// Every module the rewrite breaks at this width.
    pub(crate) breaks: Vec<Break>,
    /// How many modules the sweep was eligible to compare.
    pub(crate) candidates: usize,
    /// How many of those the original tree ran cleanly.
    pub(crate) comparable: usize,
    /// The modules whose two runs of the original differed.
    pub(crate) flaky: Vec<String>,
    /// The width, or `default` where none was pinned.
    pub(crate) label: String,
    /// How many modules the format run could not read, parse, or write.
    pub(crate) refused: usize,
    /// The modules the original tree did not run cleanly, which a run
    /// therefore never judges.
    pub(crate) uncomparable: Vec<String>,
    /// The modules a run left no record for.
    pub(crate) unmeasured: Vec<String>,
}

impl Width {
    /// How many of this width's breaks outran their deadline.
    pub(crate) fn timing_out(&self) -> usize {
        self.breaks
            .iter()
            .filter(|brk| brk.formatted.kind == Kind::Timeout)
            .count()
    }
}

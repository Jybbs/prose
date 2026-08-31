//! The records one sweep leaves, meaning what a run of a module left
//! behind, a module the rewrite breaks, one edit of a recorded fix, and one
//! width's tallies and findings.

use std::{collections::BTreeMap, ops::Range, path::Path};

use itertools::Itertools;
use prose::rule::RuleId;

/// The names the formatter reorders by design, whose value a comparison
/// therefore leaves out.
const REORDERED: [&str; 1] = ["__all__"];

/// The names every module binds through the loader rather than through its
/// own code, which a comparison leaves out.
const UNBOUND: [&str; 7] = [
    "__builtins__",
    "__cached__",
    "__doc__",
    "__file__",
    "__loader__",
    "__path__",
    "__spec__",
];

/// A module the rewrite breaks.
pub(crate) struct Break {
    /// The rules and binding the run traced it to.
    pub(crate) attribution: String,
    /// What the run from the formatted tree left behind.
    pub(crate) formatted: Outcome,
    /// The file and row it points at.
    pub(crate) frame: (String, Option<usize>),
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
    /// The file its frame names and its reason, which is what a baseline
    /// holds per break.
    pub(crate) fn key(&self) -> (String, String) {
        (self.frame.0.clone(), self.reason.clone())
    }

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

/// What a run of a module amounted to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Kind {
    /// The run bound a namespace.
    Ok,
    /// The run raised.
    Raised,
    /// The run outran its deadline.
    Timeout,
    /// The run left no record to read.
    #[default]
    Unmeasured,
}

impl Kind {
    /// The kind a probe's `kind` row names.
    fn of(named: &str) -> Self {
        match named {
            "ok" => Self::Ok,
            "raised" => Self::Raised,
            _ => Self::Unmeasured,
        }
    }
}

/// What one run of a module left behind, as the probe records it.
#[derive(Clone, Default)]
pub(crate) struct Outcome {
    /// The plain constants the run bound, each spelt.
    pub(crate) constants: BTreeMap<String, String>,
    /// The predicate of a sentence naming the module.
    pub(crate) error: String,
    /// The file and row of every frame a raise passed through.
    pub(crate) frames: Vec<(String, usize)>,
    /// What the run amounted to.
    pub(crate) kind: Kind,
    /// Every module the run took from a tree, relative to it, sorted.
    pub(crate) loaded: Vec<String>,
    /// The name a raised run could not find.
    pub(crate) name: Option<String>,
    /// The names an `ok` run bound, sorted.
    pub(crate) names: Vec<String>,
}

impl Outcome {
    /// An outcome of `kind` carrying `error` and nothing else.
    pub(crate) fn of(kind: Kind, error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            kind,
            ..Self::default()
        }
    }

    /// Reads back the record the probe wrote, which is one tagged row per
    /// line separated by `RS`, each field separated by `NUL`.
    ///
    /// The probe reports the namespace as it stands and every module path as
    /// the interpreter saw it, so the loader's own names come out here, the
    /// names sort here, and a loaded path is made relative to whichever of
    /// `trees` carries it.
    pub(crate) fn parse(record: &str, trees: &[&Path]) -> Self {
        let mut read = Self::default();
        for row in record.split('\u{1e}') {
            let mut fields = row.split('\0');
            match (fields.next(), fields.next(), fields.next()) {
                (Some("bound"), Some(name), _) if !UNBOUND.contains(&name) => {
                    read.names.push(name.to_owned());
                }
                (Some("const"), Some(name), Some(spelt))
                    if !UNBOUND.contains(&name) && !REORDERED.contains(&name) =>
                {
                    read.constants.insert(name.to_owned(), spelt.to_owned());
                }
                (Some("frame"), Some(row), Some(file)) => {
                    if let Ok(row) = row.parse() {
                        read.frames.push((file.to_owned(), row));
                    }
                }
                (Some("kind"), Some(kind), _) => read.kind = Kind::of(kind),
                (Some("loaded"), Some(path), _) => {
                    if let Some(relative) = relative_to(path, trees) {
                        read.loaded.push(relative);
                    }
                }
                (Some("missing"), Some(name), _) => read.name = Some(name.to_owned()),
                (Some("raise"), Some(raised), Some(message)) => {
                    read.error = format!("raises {raised}: {message}");
                }
                _ => {}
            }
        }
        read.loaded = read.loaded.into_iter().sorted().dedup().collect();
        read.names.sort();
        read
    }
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
    /// The modules a run left no record for.
    pub(crate) unmeasured: Vec<String>,
}

impl Width {
    /// How many candidates the original tree did not run cleanly, the
    /// unmeasured ones aside.
    pub(crate) fn uncomparable(&self) -> usize {
        self.candidates - self.comparable - self.unmeasured.len()
    }
}

/// A module path named relative to whichever of `trees` carries it, `None`
/// for one the interpreter loaded from its own library.
fn relative_to(path: &str, trees: &[&Path]) -> Option<String> {
    trees.iter().find_map(|tree| {
        Path::new(path)
            .strip_prefix(tree)
            .ok()
            .map(|relative| relative.to_string_lossy().into_owned())
    })
}

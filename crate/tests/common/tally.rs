//! Defect tallies a corpus sweep renders, keyed by wording.

use std::{
    collections::{
        BTreeMap, BTreeSet,
        btree_map::Entry::{Occupied, Vacant},
    },
    fmt::Write,
    path::Path,
};

use itertools::Itertools;

/// How many distinct defects a rendered tally prints before it reports
/// the remainder as a count.
pub(crate) const SHOWN: usize = 30;

/// What one hit of a defect carries past its wording and its file.
#[derive(Default)]
pub(crate) struct Hit {
    /// The sweep clause the hit showed under, as a label and the width
    /// the label was swept at.
    pub(crate) clause: Option<(String, usize)>,
    /// The excerpt a report shows beside the example.
    pub(crate) detail: Option<String>,
    /// The command sweeping this hit alone.
    pub(crate) repro: Option<String>,
}

/// The defects one corpus sweep found, each keyed by its own wording so
/// the same shape across many files and sweep clauses reports once.
#[derive(Default)]
pub(crate) struct Tally(BTreeMap<String, Site>);

impl Tally {
    fn place(&mut self, defect: String, site: Site) {
        match self.0.entry(defect) {
            Occupied(mut held) => held.get_mut().merge(site),
            Vacant(slot) => {
                slot.insert(site);
            }
        }
    }

    /// Folds `other` in, merging the files and clauses of a defect both
    /// sides carry and keeping whichever example sorts first.
    pub(crate) fn absorb(&mut self, other: Self) {
        for (defect, site) in other.0 {
            self.place(defect, site);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Files a defect against `path`, `hit` carrying the clause it
    /// showed under, the command that sweeps it alone, and the excerpt a
    /// report shows beside it.
    pub(crate) fn record_hit(&mut self, defect: String, path: &Path, hit: Hit) {
        self.place(defect, Site::single(path, hit));
    }

    /// Renders the defects under `heading`, capped at [`SHOWN`] with the
    /// remainder reported as a count. Empty for an empty tally, so a
    /// caller concatenates several and tests the result for emptiness.
    pub(crate) fn render(&self, heading: &str) -> String {
        if self.0.is_empty() {
            return String::new();
        }
        let shown = self
            .0
            .iter()
            .take(SHOWN)
            .map(|(defect, site)| site.render(defect))
            .format("\n");
        let rest = self.0.len().saturating_sub(SHOWN);
        let tail = if rest > 0 {
            format!("\n  ... and {rest} more")
        } else {
            String::new()
        };
        format!("\n{heading} ({}):\n{shown}{tail}", self.0.len())
    }
}

/// The hit a rendered defect names.
struct Example {
    clause: Option<String>,
    detail: Option<String>,
    file: String,
    repro: Option<String>,
}

impl Example {
    fn key(&self) -> (&str, Option<&str>) {
        (&self.file, self.clause.as_deref())
    }
}

/// The files and clauses a defect reached, with the example a report
/// names.
struct Site {
    /// Every clause label the defect reached, with the widths under it.
    clauses: BTreeMap<String, BTreeSet<usize>>,
    example: Example,
    files: BTreeSet<String>,
}

impl Site {
    fn single(path: &Path, hit: Hit) -> Self {
        let file = path.display().to_string();
        let clause = hit
            .clause
            .as_ref()
            .map(|(label, width)| format!("{label} {width}"));
        let clauses = hit
            .clause
            .into_iter()
            .map(|(label, width)| (label, BTreeSet::from([width])))
            .collect();
        Self {
            clauses,
            example: Example {
                clause,
                detail: hit.detail,
                file: file.clone(),
                repro: hit.repro,
            },
            files: BTreeSet::from([file]),
        }
    }

    /// Folds `other` in, the example that sorts first surviving so a
    /// rerun names the same one whatever order the sweep reached them.
    fn merge(&mut self, other: Self) {
        self.files.extend(other.files);
        for (label, widths) in other.clauses {
            self.clauses.entry(label).or_default().extend(widths);
        }
        if other.example.key() < self.example.key() {
            self.example = other.example;
        }
    }

    fn render(&self, defect: &str) -> String {
        let plural = if self.files.len() == 1 { "" } else { "s" };
        let at = self
            .example
            .clause
            .as_ref()
            .map_or_else(String::new, |clause| format!(" at {clause}"));
        let mut rendered = format!(
            "  {defect} ({} file{plural}, e.g. {}{at})",
            self.files.len(),
            self.example.file
        );
        if self.clauses.values().map(BTreeSet::len).sum::<usize>() > 1 {
            let reached = self
                .clauses
                .iter()
                .map(|(label, widths)| format!("{label} {}", widths.iter().format(", ")))
                .format(" and ");
            let _ = write!(rendered, "\n    reached at {reached}");
        }
        if let Some(cmd) = &self.example.repro {
            let _ = write!(rendered, "\n    reproduce with {cmd}");
        }
        if let Some(detail) = &self.example.detail {
            for line in detail.lines() {
                let _ = write!(rendered, "\n    {line}");
            }
        }
        rendered
    }
}

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
pub const SHOWN: usize = 30;

/// The clause a capped listing closes on, empty where nothing was cut.
pub fn remainder(total: usize) -> String {
    match total.saturating_sub(SHOWN) {
        0 => String::new(),
        rest => format!("\n  ... and {rest} more"),
    }
}

/// `count` further items of `noun`, written as `1 more {noun}` or
/// `N more {noun}s`.
pub fn more(count: usize, noun: &str) -> String {
    match count {
        1 => format!("1 more {noun}"),
        _ => format!("{count} more {noun}s"),
    }
}

/// `first` beside how many more followed it, written as `and 1 more {noun}`
/// or `and N more {noun}s`.
pub fn with_rest(first: &str, rest: usize, noun: &str) -> String {
    match rest {
        0 => first.to_owned(),
        _ => format!("{first} and {}", more(rest, noun)),
    }
}

/// What one hit of a defect carries past its wording and its file.
#[derive(Default)]
pub struct Hit {
    /// The sweep clause the hit showed under, as a label and the width
    /// the label was swept at.
    pub clause: Option<(String, usize)>,
    /// The excerpt a report shows beside the example.
    pub detail: Option<String>,
    /// The command sweeping this hit alone.
    pub repro: Option<String>,
}

/// The defects one corpus sweep found, each keyed by its own wording so
/// the same shape across many files and sweep clauses reports once.
#[derive(Default)]
pub struct Tally(BTreeMap<String, Site>);

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
    pub fn absorb(&mut self, other: Self) {
        for (defect, site) in other.0 {
            self.place(defect, site);
        }
    }

    /// Whether the sweep found no defect.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Files a defect against `path`, `hit` carrying the clause it
    /// showed under, the command that sweeps it alone, and the excerpt a
    /// report shows beside it.
    pub fn record_hit(&mut self, defect: String, path: &Path, hit: Hit) {
        self.place(defect, Site::single(path, hit));
    }

    /// Renders the defects under `heading`, capped at [`SHOWN`] with the
    /// remainder reported as a count. Empty for an empty tally, so a
    /// caller concatenates several and tests the result for emptiness.
    pub fn render(&self, heading: &str) -> String {
        if self.is_empty() {
            return String::new();
        }
        let shown = self
            .0
            .iter()
            .take(SHOWN)
            .map(|(defect, site)| site.render(defect))
            .format("\n");
        let tail = remainder(self.0.len());
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn tally_names_a_defect_once_across_clauses_and_keeps_the_earliest_example() {
        let hit = |label: &str, width| Hit {
            clause: Some((label.to_owned(), width)),
            ..Hit::default()
        };
        let mut tally = Tally::default();
        tally.record_hit(
            "still editing".to_owned(),
            Path::new("b.py"),
            hit("code", 88),
        );
        tally.record_hit(
            "still editing".to_owned(),
            Path::new("a.py"),
            hit("import", 60),
        );
        tally.record_hit(
            "still editing".to_owned(),
            Path::new("a.py"),
            hit("code", 40),
        );

        let rendered = tally.render("defects");

        assert_eq!(tally.len(), 1);
        assert!(
            rendered.contains("still editing (2 files, e.g. a.py at code 40)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("reached at code 40, 88 and import 60"),
            "{rendered}"
        );
    }

    #[test]
    fn tally_render_caps_the_defects_it_prints_and_counts_the_rest() {
        let mut tally = Tally::default();
        for n in 0..=SHOWN {
            tally.record_hit(format!("defect {n:02}"), Path::new("a.py"), Hit::default());
        }

        let rendered = tally.render("defects");

        assert!(
            rendered.contains(&format!("defect {:02}", SHOWN - 1)),
            "{rendered}"
        );
        assert!(
            !rendered.contains(&format!("defect {SHOWN:02}")),
            "{rendered}"
        );
        assert!(rendered.ends_with("... and 1 more"), "{rendered}");
    }

    #[test]
    fn tally_render_carries_the_example_repro_and_detail() {
        let mut tally = Tally::default();
        let hit = Hit {
            detail: Some("--- a\n+++ b".to_owned()),
            repro: Some("cargo test".to_owned()),
            ..Hit::default()
        };
        tally.record_hit("still editing".to_owned(), Path::new("a.py"), hit);

        let rendered = tally.render("defects");

        assert!(
            rendered.contains("still editing (1 file, e.g. a.py)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("\n    reproduce with cargo test"),
            "{rendered}"
        );
        assert!(rendered.ends_with("\n    --- a\n    +++ b"), "{rendered}");
    }
}

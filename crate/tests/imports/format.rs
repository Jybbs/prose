//! Formatting a copy of the corpus in place and recording which rows each
//! safe fix rewrote, which is what an attribution reads back.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    path::{Path, PathBuf},
};

use prose::{pipeline::Pipeline, source::Source};
use ruff_diagnostics::Applicability;
use ruff_source_file::LineIndex;
use ruff_text_size::{Ranged, TextSize};

use crate::{
    common::{Absorbing, Slot, python_files, swept},
    records::{EditRows, Fixes},
};

/// What formatting one tree in place left behind, each file named relative
/// to the tree.
#[derive(Default)]
pub(crate) struct Formatted {
    /// The safe fixes each file's run recorded.
    pub(crate) fixes: Fixes,
    /// Every file the pipeline read.
    pub(crate) read: BTreeSet<String>,
    /// Each file the pipeline read and could not format, beside the error it
    /// returned.
    pub(crate) rejected: BTreeMap<String, String>,
    /// How many files the run rewrote.
    pub(crate) rewritten: usize,
    /// How many files the pipeline could not read or parse.
    pub(crate) unread: usize,
}

impl Absorbing for Formatted {
    fn absorb(&mut self, other: Self) {
        self.fixes.extend(other.fixes);
        self.read.extend(other.read);
        self.rejected.extend(other.rejected);
        self.rewritten += other.rewritten;
        self.unread += other.unread;
    }
}

/// The original rows one edit rewrote, an end at column 1 closing on the row
/// above it.
pub(crate) fn edit_rows(lines: &LineIndex, text: &str, range: &Range<usize>) -> Range<usize> {
    let start = row_of(lines, range.start);
    let closing = lines.line_index(offset(range.end));
    let mut end = closing.get();
    if end > start && usize::from(lines.line_start(closing, text)) == range.end {
        end -= 1;
    }
    start..end + 1
}

/// Formats every file of a tree in place and returns the files it read and
/// rejected, how many it rewrote, the safe fixes each file's run recorded,
/// and how many files it could not read or parse.
///
/// A file the pipeline could not read, parse, or format is left as it was,
/// an unread or unparsed file only counted and a file the pipeline could not
/// format named beside its error.
pub(crate) fn format_tree(tree: &Path, pipeline: &Pipeline) -> Formatted {
    let files: Vec<PathBuf> = python_files(tree).collect();
    swept(&files, |path| formatted(path, pipeline, tree))
}

/// The row `at` sits on, counting from one.
pub(crate) fn row_of(lines: &LineIndex, at: usize) -> usize {
    lines.line_index(offset(at)).get()
}

/// What formatting one file of `tree` in place left behind, recording one
/// unread file and nothing else where the pipeline could not read or parse
/// it.
fn formatted(path: &Path, pipeline: &Pipeline, tree: &Path) -> Formatted {
    let _slot = Slot::open(path.display().to_string());
    let Ok(source) = Source::from_path(path) else {
        return Formatted {
            unread: 1,
            ..Formatted::default()
        };
    };
    let module = path
        .strip_prefix(tree)
        .unwrap_or_else(|_| unreachable!("invariant: the walk is rooted at the tree"))
        .to_string_lossy()
        .into_owned();
    let text = source.text().to_owned();
    let lines = LineIndex::from_source_text(&text);
    let diagnostics = pipeline.diagnose(&source);
    let read = BTreeSet::from([module.clone()]);
    let written = match pipeline.format(source) {
        Ok(written) => written,
        Err(error) => {
            return Formatted {
                read,
                rejected: BTreeMap::from([(module, error.to_string())]),
                ..Formatted::default()
            };
        }
    };
    let changed = written.text() != text;
    if changed {
        fs_err::write(path, written.text()).expect("write a formatted file into the stage");
    }
    let fixes: Vec<_> = diagnostics
        .into_iter()
        .filter_map(|diagnostic| {
            let fix = diagnostic.fix?;
            (fix.applicability() == Applicability::Safe).then(|| {
                let edits = fix
                    .edits()
                    .iter()
                    .map(|edit| {
                        let range = usize::from(edit.start())..usize::from(edit.end());
                        EditRows {
                            content: edit.content().unwrap_or_default().to_owned(),
                            rows: edit_rows(&lines, &text, &range),
                            range,
                        }
                    })
                    .collect();
                (diagnostic.rule, edits)
            })
        })
        .collect();
    Formatted {
        fixes: Fixes::from_iter((!fixes.is_empty()).then(|| (module.clone(), fixes))),
        read,
        rewritten: usize::from(changed),
        ..Formatted::default()
    }
}

/// The byte offset as the size a line index reads.
fn offset(at: usize) -> TextSize {
    TextSize::try_from(at).expect("a corpus module fits a text size")
}

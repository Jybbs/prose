//! Formatting a copy of the corpus in place and recording which rows each
//! safe fix rewrote, which is what an attribution reads back.

use std::{
    collections::BTreeSet,
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

/// What formatting one tree in place left behind.
#[derive(Default)]
pub(crate) struct Formatted {
    /// The safe fixes each file's run recorded.
    pub(crate) fixes: Fixes,
    /// How many modules the pipeline could not read, parse, or write.
    pub(crate) refused: usize,
    /// The modules the run rewrote, each named relative to the tree.
    pub(crate) rewritten: BTreeSet<String>,
}

impl Absorbing for Formatted {
    fn absorb(&mut self, other: Self) {
        self.fixes.extend(other.fixes);
        self.refused += other.refused;
        self.rewritten.extend(other.rewritten);
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

/// Formats every module of a tree in place and returns the modules it
/// rewrote and the safe fixes each file's run recorded, beside how many
/// modules the pipeline refused.
///
/// A module the pipeline refuses is left as it was and counted, since a run
/// that quietly formats less than it walked reports fewer breaks for a reason
/// that never reaches the report.
pub(crate) fn format_tree(tree: &Path, pipeline: &Pipeline) -> Formatted {
    let files: Vec<PathBuf> = python_files(tree).collect();
    swept(&files, |path| formatted(path, pipeline, tree))
}

/// The row `at` sits on, counting from one.
pub(crate) fn row_of(lines: &LineIndex, at: usize) -> usize {
    lines.line_index(offset(at)).get()
}

/// What formatting one module of `tree` in place left behind, a module the
/// pipeline refuses counting itself and recording no fix.
fn formatted(path: &Path, pipeline: &Pipeline, tree: &Path) -> Formatted {
    let refused = Formatted {
        refused: 1,
        ..Formatted::default()
    };
    let _slot = Slot::open(path.display().to_string());
    let relative = path
        .strip_prefix(tree)
        .unwrap_or_else(|_| unreachable!("invariant: the walk is rooted at the tree"));
    let Ok(source) = Source::from_path(path) else {
        return refused;
    };
    let text = source.text().to_owned();
    let lines = LineIndex::from_source_text(&text);
    let diagnostics = pipeline.diagnose(&source);
    let Ok((written, _)) = pipeline.run(source) else {
        return refused;
    };
    let changed = written.text() != text;
    if changed && fs_err::write(path, written.text()).is_err() {
        return refused;
    }
    let module = relative.to_string_lossy().into_owned();
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
        refused: 0,
        rewritten: changed.then_some(module).into_iter().collect(),
    }
}

/// The byte offset as the size a line index reads.
fn offset(at: usize) -> TextSize {
    TextSize::try_from(at).expect("a corpus module fits a text size")
}

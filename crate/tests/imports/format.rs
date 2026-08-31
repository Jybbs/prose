//! Formatting a copy of the corpus in place and recording which rows each
//! safe fix rewrote, which is what an attribution reads back.

use std::{
    ops::Range,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use prose::{pipeline::Pipeline, source::Source};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use ruff_diagnostics::Applicability;
use ruff_source_file::LineIndex;
use ruff_text_size::{Ranged, TextSize};

use crate::{
    corpus::modules_under,
    records::{EditRows, Fixes},
};

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

/// Formats every module of a tree in place and returns the safe fixes each
/// file's run recorded, beside how many modules the pipeline refused.
///
/// A module the pipeline refuses is left as it was and counted, since a run
/// that quietly formats less than it walked reports fewer breaks for a reason
/// that never reaches the report.
pub(crate) fn format_tree(tree: &Path, pipeline: &Pipeline) -> (Fixes, usize) {
    let refused = AtomicUsize::new(0);
    let fixes = modules_under(tree)
        .par_iter()
        .filter_map(|relative| {
            let path = tree.join(relative);
            let Ok(source) = Source::from_path(&path) else {
                refused.fetch_add(1, Ordering::Relaxed);
                return None;
            };
            let text = source.text().to_owned();
            let lines = LineIndex::from_source_text(&text);
            let diagnostics = pipeline.diagnose(&source);
            let Ok((formatted, _)) = pipeline.run(source) else {
                refused.fetch_add(1, Ordering::Relaxed);
                return None;
            };
            if formatted.text() != text && fs_err::write(&path, formatted.text()).is_err() {
                refused.fetch_add(1, Ordering::Relaxed);
                return None;
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
                                let range = usize::from(edit.range().start())
                                    ..usize::from(edit.range().end());
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
            (!fixes.is_empty()).then(|| (relative.clone(), fixes))
        })
        .collect();
    (fixes, refused.load(Ordering::Relaxed))
}

/// The row `at` sits on, counting from one.
pub(crate) fn row_of(lines: &LineIndex, at: usize) -> usize {
    lines.line_index(offset(at)).get()
}

/// The byte offset as the size a line index reads.
fn offset(at: usize) -> TextSize {
    TextSize::try_from(at).expect("a corpus module fits a text size")
}

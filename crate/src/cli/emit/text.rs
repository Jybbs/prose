//! Text emitter: rustc-style snippet rendering with carets and fix
//! suggestions.

use std::io::{self, Write};

use annotate_snippets::{AnnotationKind, Level, Patch, Renderer, Snippet};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use ruff_diagnostics::Fix;
use ruff_notebook::NotebookIndex;
use ruff_source_file::{OneIndexed, SourceFile};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use super::{Emitter, EmitterSummary, Run};
use crate::diagnostics::Diagnostic;

/// Rows of context the renderer draws either side of a diagnostic.
const CONTEXT_LINES: usize = 2;

pub(crate) struct Text {
    renderer: Renderer,
}

impl Text {
    /// Builds the emitter, styling its renderer only where the stream
    /// carries escapes through.
    pub(crate) fn new(color: bool) -> Self {
        Self {
            renderer: if color {
                Renderer::styled()
            } else {
                Renderer::plain()
            },
        }
    }

    /// Renders one file's diagnostics into an owned buffer.
    pub(in crate::cli) fn render_run(&self, run: &Run<'_>) -> io::Result<Vec<u8>> {
        let path = run.file.name();
        let mut out = Vec::new();
        for diag in run.diagnostics {
            let (base, header, text, line_start) =
                view(run.file, run.notebook_index, annotated(diag));
            if let Some(header) = header {
                writeln!(out, "{header}")?;
            }
            let shift = |range: TextRange| (range - base).to_std_range();
            let warning = Level::WARNING.primary_title(diag.message.as_str()).element(
                framed(text, line_start, path).annotation(
                    AnnotationKind::Primary
                        .span(shift(diag.range))
                        .label(diag.rule.as_str()),
                ),
            );
            let mut groups = vec![warning];
            if let Some(fix) = &diag.fix {
                let snippet =
                    framed(text, line_start, path).patches(fix.edits().iter().map(|edit| {
                        Patch::new(shift(edit.range()), edit.content().unwrap_or_default())
                    }));
                groups.push(Level::HELP.secondary_title("replace with").element(snippet));
            }
            writeln!(out, "{}", self.renderer.render(&groups))?;
        }
        Ok(out)
    }
}

impl Emitter for Text {
    /// Renders each run on the rayon pool, then writes the buffers back
    /// in source order, which rayon's indexed `collect` preserves.
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        _summary: &EmitterSummary,
    ) -> io::Result<()> {
        let blocks: Vec<Vec<u8>> = runs
            .par_iter()
            .map(|run| self.render_run(run))
            .collect::<io::Result<_>>()?;
        blocks.iter().try_for_each(|block| writer.write_all(block))
    }
}

/// The source span a diagnostic renders against, its own range widened
/// to every edit its fix would apply, so the snippet holds each patch
/// the renderer draws.
fn annotated(diag: &Diagnostic) -> TextRange {
    diag.fix
        .iter()
        .flat_map(Fix::edits)
        .fold(diag.range, |span, edit| span.cover(edit.range()))
}

/// The notebook cells `range` runs across, paired with the byte range
/// they span, derived from the index by walking from the first cell's
/// row to the row opening the cell past the last. A rule reads the
/// cells concatenated, so a range covering an aligned group spans
/// every cell that group reaches. `None` for an offset outside every
/// cell.
fn cell_slice(
    file: &SourceFile,
    index: &NotebookIndex,
    range: TextRange,
) -> Option<(OneIndexed, OneIndexed, TextRange)> {
    let code = file.to_source_code();
    let first = index.cell(code.line_column(range.start()).line)?;
    let last = index.cell(code.line_column(range.end()).line)?;
    let mut start = None;
    let mut end = file.source_text().text_len();
    for cell_start in index.iter() {
        let byte = code.line_start(cell_start.start_row());
        if cell_start.cell_index() > last {
            end = byte;
            break;
        }
        if cell_start.cell_index() == first {
            start = Some(byte);
        }
    }
    start.map(|start| (first, last, TextRange::new(start, end)))
}

/// The snippet frame both the warning and the fix suggestion draw on,
/// `text` numbered from `line_start` under `path`.
fn framed<'a, T: Clone>(text: &'a str, line_start: usize, path: &'a str) -> Snippet<'a, T> {
    Snippet::source(text).line_start(line_start).path(path)
}

/// The snippet view for a diagnostic spanning `range`: the byte offset
/// its rendered text begins at, a cell header for a notebook, that text,
/// and the one-based line its first row carries. A module renders the
/// lines around `range` with no header and their own numbering, where a
/// notebook renders the cells the range reaches, from line one, under a
/// header naming them.
fn view<'a>(
    file: &'a SourceFile,
    index: Option<&NotebookIndex>,
    range: TextRange,
) -> (TextSize, Option<String>, &'a str, usize) {
    let source = file.source_text();
    match index.and_then(|index| cell_slice(file, index, range)) {
        Some((first, last, span)) => (span.start(), Some(header(first, last)), &source[span], 1),
        None => {
            let (span, first) = window(file, range);
            (span.start(), None, &source[span], first)
        }
    }
}

/// The header a notebook snippet draws above its cells, reading as
/// `cell 3` for one and `cells 3 to 5` for a range.
fn header(first: OneIndexed, last: OneIndexed) -> String {
    if first == last {
        format!("cell {first}")
    } else {
        format!("cells {first} to {last}")
    }
}

/// The line-bounded slice of `file` covering `range` with `CONTEXT_LINES`
/// rows either side, paired with the one-based number of its first row.
/// Handing the renderer this window rather than the whole file keeps a
/// file's rendering cost off its diagnostic count.
fn window(file: &SourceFile, range: TextRange) -> (TextRange, usize) {
    let code = file.to_source_code();
    let first = code.line_index(range.start()).saturating_sub(CONTEXT_LINES);
    let last = code
        .line_index(range.end())
        .saturating_add(CONTEXT_LINES)
        .min(OneIndexed::from_zero_indexed(
            code.line_count().saturating_sub(1),
        ));
    (
        TextRange::new(code.line_start(first), code.line_end(last)),
        first.get(),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Fix;

    use super::*;
    use crate::{
        cli::emit::{emitted, emitted_runs},
        diagnostics::Diagnostic,
        source::Source,
        testing::{format_diagnostic, notebook, notebook_index, parse, range, replacement},
    };

    /// A two-cell notebook whose comment column aligns across the cell
    /// boundary, the shape a rule reading the concatenated source
    /// produces.
    const COMMENTED_CELLS: [&str; 2] = ["x = 1  # a\n", "yy = 2  # bb\n"];

    /// Emits `sources` as one run apiece, each carrying a diagnostic
    /// over its first three bytes, and returns the rendered stream.
    fn emit_runs(sources: &[Source], color: bool) -> String {
        let diag = format_diagnostic(range(0, 3));
        let runs: Vec<Run<'_>> = sources
            .iter()
            .map(|s| Run::new(s.source_file(), std::slice::from_ref(&diag), None))
            .collect();
        let buf = emitted_runs(&Text::new(color), &runs, &EmitterSummary::default());
        String::from_utf8(buf).expect("utf-8")
    }

    fn render_to_string(source: &Source, diag: &Diagnostic) -> String {
        let buf = emitted(
            &Text::new(false),
            source.source_file(),
            std::slice::from_ref(diag),
            &EmitterSummary::default(),
        );
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn appends_help_block_when_fix_is_available() {
        let source = parse("x = 1\n");
        let rendered = render_to_string(&source, &format_diagnostic(range(0, 1)));
        assert!(rendered.contains("warning: rewrite x to y"));
        assert!(rendered.contains("help: replace with"));
        assert!(rendered.contains('y'));
    }

    #[test]
    fn emit_writes_each_runs_block_in_source_order() {
        let sources = [parse("aaa = 1\n"), parse("bbb = 2\n"), parse("ccc = 3\n")];

        let rendered = emit_runs(&sources, false);

        let seats: Vec<usize> = ["aaa", "bbb", "ccc"]
            .iter()
            .map(|name| rendered.find(name).expect("every run renders"))
            .collect();
        assert!(seats.is_sorted());
    }

    #[test]
    fn help_block_renders_every_edit_in_a_group() {
        let source = parse("x = 1\ny = 2\n");
        let rendered = render_to_string(
            &source,
            &Diagnostic {
                fix: Some(Fix::safe_edits(
                    replacement("aaa", 0, 1),
                    [replacement("bbb", 6, 7)],
                )),
                ..format_diagnostic(range(0, 7))
            },
        );
        assert!(rendered.contains("help: replace with"));
        assert!(rendered.contains("aaa"));
        assert!(rendered.contains("bbb"));
    }

    #[rstest]
    #[case::inside_one_cell(range(13, 15), "cell 2")]
    #[case::across_two_cells(range(7, 20), "cells 1 to 2")]
    fn notebook_snippet_names_every_cell_its_range_reaches(
        #[case] span: TextRange,
        #[case] expected: &str,
    ) {
        let source = notebook(&COMMENTED_CELLS);
        let index = notebook_index(&COMMENTED_CELLS);
        let diag = format_diagnostic(span);

        let buf = emitted_runs(
            &Text::new(false),
            &[Run::new(
                source.source_file(),
                std::slice::from_ref(&diag),
                Some(&index),
            )],
            &EmitterSummary::default(),
        );

        let rendered = String::from_utf8(buf).expect("utf-8");
        assert!(rendered.contains(expected), "rendered {rendered:?}");
    }

    #[rstest]
    #[case::styled(true, true)]
    #[case::plain(false, false)]
    fn renderer_paints_the_snippet_only_under_color(#[case] color: bool, #[case] painted: bool) {
        let rendered = emit_runs(std::slice::from_ref(&parse("aaa = 1\n")), color);

        assert_eq!(rendered.contains('\u{1b}'), painted);
    }

    #[test]
    fn renders_path_line_column_message_and_caret() {
        let source = parse("x = 1\n");
        let rendered = render_to_string(
            &source,
            &Diagnostic {
                fix: None,
                ..format_diagnostic(range(0, 1))
            },
        );
        assert!(rendered.contains("warning: rewrite x to y"));
        assert!(rendered.contains("--> <source>:1:1"));
        assert!(rendered.contains("rewrite-x"));
        assert!(rendered.contains("x = 1"));
    }
}

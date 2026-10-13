//! Unified-diff rendering for `format --diff`.

use std::io::Write;

use anyhow::Context;
use itertools::Itertools;
use ruff_notebook::NotebookIndex;

use crate::{
    cache::{NotebookRewrite, RewriteKind},
    cli::output::{self, Presentation},
};

/// How each file's diff is headed.
#[derive(Clone, Copy, Debug)]
pub(super) enum Heading {
    /// A `---` and `+++` pair naming the two passes a settle report
    /// diffs.
    Passes,
    /// The plain `---` and `+++` pair.
    Patch,
    /// A `🧵 <name>` line, painted Ube when `color`.
    Thread { color: bool },
}

impl From<&Presentation> for Heading {
    /// Heads each file with the `🧵` line on a non-quiet TTY, painting it
    /// only under color, and with the plain patch header elsewhere.
    fn from(present: &Presentation) -> Self {
        if present.decorate_diff() {
            Self::Thread {
                color: present.color,
            }
        } else {
            Self::Patch
        }
    }
}

/// Writes a unified diff between `before` and `after`.
pub(super) fn write_diff<W: Write>(
    writer: &mut W,
    name: &str,
    before: &str,
    after: &str,
    heading: Heading,
) -> anyhow::Result<()> {
    let diff = similar::TextDiff::configure()
        .algorithm(similar::Algorithm::Histogram)
        .diff_lines(before, after);
    let mut unified = diff.unified_diff();
    match heading {
        Heading::Passes => {
            unified.header(
                &format!("{name} (first pass)"),
                &format!("{name} (second pass)"),
            );
        }
        Heading::Patch => {
            unified.header(name, name);
        }
        Heading::Thread { color } => {
            let line = format!("🧵 {name}");
            let line = if color { output::ube(&line) } else { line };
            writeln!(writer, "{line}").context("writing diff")?;
        }
    }
    unified.to_writer(writer).context("writing diff")?;
    Ok(())
}

/// Writes the diff for a `Changed` rewrite: per code cell for a
/// notebook, one unified diff for a module. A notebook carries an
/// `index` for its per-cell headers, a module `None`.
pub(super) fn write_rewrite_diff<W: Write>(
    writer: &mut W,
    name: &str,
    before: &str,
    kind: &RewriteKind,
    index: Option<&NotebookIndex>,
    heading: Heading,
) -> anyhow::Result<()> {
    match kind {
        RewriteKind::Notebook(notebook) => write_notebook_diff(
            writer,
            name,
            notebook,
            index.expect("a notebook rewrite carries its cell index"),
            heading,
        ),
        RewriteKind::Text(code) => write_diff(writer, name, before, code, heading),
    }
}

/// Writes a per-cell unified diff for a notebook, one cell header and
/// hunk set per code cell whose source changed. `index` numbers each
/// header by the cell's absolute notebook position, the number the
/// diagnostics carry.
fn write_notebook_diff<W: Write>(
    writer: &mut W,
    name: &str,
    rewrite: &NotebookRewrite,
    index: &NotebookIndex,
    heading: Heading,
) -> anyhow::Result<()> {
    for ((before, after), cell_start) in rewrite
        .before
        .iter()
        .zip_eq(&rewrite.after)
        .zip_eq(index.iter())
    {
        if before == after {
            continue;
        }
        let cell = format!("{name} cell {}", cell_start.cell_index());
        write_diff(writer, &cell, before, after, heading)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

    const AFTER: &str = "ab = 1\nx  = 2\n";
    const BEFORE: &str = "ab = 1\nx = 2\n";

    fn rendered(heading: Heading) -> String {
        crate::testing::rendered(|buf| {
            write_diff(buf, "sample.py", BEFORE, AFTER, heading).expect("writes");
        })
    }

    #[test]
    fn from_presentation_falls_back_to_the_patch_header_off_a_tty() {
        assert_matches!(Heading::from(&Presentation::windowed()), Heading::Patch);
    }

    #[rstest]
    #[case::color_on(true, true)]
    #[case::color_off(false, false)]
    fn from_presentation_paints_the_thread_line_only_under_color(
        #[case] color: bool,
        #[case] painted: bool,
    ) {
        let present = Presentation {
            color,
            quiet: false,
            stderr_color: color,
            stdout_tty: true,
        };

        assert_matches!(
            Heading::from(&present),
            Heading::Thread { color } if color == painted
        );
    }

    #[test]
    fn write_diff_leaves_the_thread_anchor_unpainted_without_color() {
        let out = rendered(Heading::Thread { color: false });

        assert!(out.contains("🧵 sample.py"), "anchor missing: {out:?}");
        assert!(!out.contains('\u{1b}'), "paint leaked: {out:?}");
        assert!(!out.contains("--- "), "plain header leaked: {out:?}");
    }

    #[test]
    fn write_diff_paints_the_thread_anchor_under_color() {
        let out = rendered(Heading::Thread { color: true });

        assert!(out.contains("🧵 sample.py"), "anchor missing: {out:?}");
        assert!(out.contains('\u{1b}'), "paint missing: {out:?}");
        assert!(out.contains("@@"), "hunks missing: {out:?}");
    }

    #[test]
    fn write_diff_plain_keeps_the_patch_header() {
        let out = rendered(Heading::Patch);

        assert!(
            out.contains("--- sample.py"),
            "patch header missing: {out:?}"
        );
        assert!(!out.contains('🧵'), "decoration leaked: {out:?}");
    }
}

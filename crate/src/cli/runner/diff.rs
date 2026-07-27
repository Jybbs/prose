//! Unified-diff rendering for `format --diff`.

use std::io::Write;

use anyhow::Context;
use itertools::izip;
use ruff_notebook::NotebookIndex;

use crate::{
    cache::{NotebookRewrite, RewriteKind},
    cli::output,
};

/// How each file's diff is headed.
#[derive(Clone, Copy, Debug)]
pub(super) enum Heading {
    /// The plain `---` and `+++` pair.
    Patch,
    /// A `🧵 <name>` line, painted Ube when `color`.
    Thread { color: bool },
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

/// Writes a unified diff between `before` and `after`.
fn write_diff<W: Write>(
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
    for (before, after, cell_start) in izip!(&rewrite.before, &rewrite.after, index.iter()) {
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
    use super::*;

    const AFTER: &str = "ab = 1\nx  = 2\n";
    const BEFORE: &str = "ab = 1\nx = 2\n";

    fn rendered(heading: Heading) -> String {
        let mut buf = Vec::new();
        write_diff(&mut buf, "sample.py", BEFORE, AFTER, heading).expect("writes");
        String::from_utf8(buf).expect("utf-8")
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

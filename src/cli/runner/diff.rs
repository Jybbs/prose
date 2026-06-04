//! Unified-diff rendering for `format --diff`.

use std::io::Write;

use anyhow::Context;

use crate::cli::output::{self};

/// Writes a unified diff between `before` and `after`. When
/// `decorate`, a Ube `🧵 <name>` heading stands in for the plain
/// `---`/`+++` header, which is reserved for off-TTY runs so the diff
/// stays a valid patch.
pub(super) fn write_diff<W: Write>(
    writer: &mut W,
    name: &str,
    before: &str,
    after: &str,
    decorate: bool,
) -> anyhow::Result<()> {
    let diff = similar::TextDiff::from_lines(before, after);
    let mut unified = diff.unified_diff();
    if decorate {
        writeln!(writer, "{}", output::ube(&format!("🧵 {name}"))).context("writing diff")?;
    } else {
        unified.header(name, name);
    }
    unified.to_writer(writer).context("writing diff")?;
    Ok(())
}

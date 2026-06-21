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
    let diff = similar::TextDiff::configure()
        .algorithm(similar::Algorithm::Histogram)
        .diff_lines(before, after);
    let mut unified = diff.unified_diff();
    if decorate {
        writeln!(writer, "{}", output::ube(&format!("🧵 {name}"))).context("writing diff")?;
    } else {
        unified.header(name, name);
    }
    unified.to_writer(writer).context("writing diff")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_diff_decorates_with_thread_anchor() {
        let mut buf = Vec::new();
        {
            let mut writer = anstream::AutoStream::never(&mut buf);
            write_diff(
                &mut writer,
                "sample.py",
                "ab = 1\nx = 2\n",
                "ab = 1\nx  = 2\n",
                true,
            )
            .expect("writes");
        }
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(out.contains("🧵 sample.py"), "anchor missing: {out:?}");
        assert!(!out.contains("--- "), "plain header leaked: {out:?}");
        assert!(out.contains("@@"), "hunks missing: {out:?}");
    }

    #[test]
    fn write_diff_plain_keeps_the_patch_header() {
        let mut buf = Vec::new();
        write_diff(
            &mut buf,
            "sample.py",
            "ab = 1\nx = 2\n",
            "ab = 1\nx  = 2\n",
            false,
        )
        .expect("writes");
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(
            out.contains("--- sample.py"),
            "patch header missing: {out:?}"
        );
        assert!(!out.contains('🧵'), "decoration leaked: {out:?}");
    }
}

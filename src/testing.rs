//! Helpers shared across `#[cfg(test)] mod tests` blocks.

use std::path::Path;

use ruff_text_size::TextRange;

use crate::source::Source;

pub(crate) fn assert_send_sync<T: Send + Sync>() {}

pub(crate) fn parse(src: &str) -> Source {
    src.parse().expect("test source parses")
}

pub(crate) fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
}

pub(crate) fn write_prose_toml(dir: &Path, contents: &str) {
    std::fs::write(dir.join("prose.toml"), contents).expect("prose.toml writes");
}

pub(crate) fn write_pyproject(dir: &Path, contents: &str) {
    std::fs::write(dir.join("pyproject.toml"), contents).expect("pyproject writes");
}

//! Plain-data cache records serialized into each entry file.

use std::{path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::diagnostics::Diagnostic;

/// Post-pipeline state cached per `(source, config, rules, version)`
/// key. The diagnostics are always anchored to the source as written,
/// leaving any mode free to render them. The rewrite is `Skipped` unless
/// the writing mode ran [`Pipeline::run`](crate::pipeline::Pipeline::run).
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CacheEntry {
    pub diagnostics: Vec<Diagnostic>,
    pub rewrite: Rewrite,
}

/// Snapshot of the cache directory's contents at one point in time.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CacheInfo {
    pub bytes: u64,
    pub entries: usize,
    pub newest_mtime: Option<SystemTime>,
    pub oldest_mtime: Option<SystemTime>,
    pub path: PathBuf,
}

/// Outcome of a `Cache::clean` or `Cache::compact` call.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CleanReport {
    pub bytes: u64,
    pub entries: usize,
}

impl CleanReport {
    /// Records one removed file of `bytes`.
    pub(super) fn record(&mut self, bytes: u64) {
        self.bytes += bytes;
        self.entries += 1;
    }
}

/// What a mode knows about the file's rewrite. `Skipped` marks a mode
/// that never computed the rewrite, whereas the other two record a
/// completed `run`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rewrite {
    /// `run` produced output differing from the original, carried as a
    /// kind that knows how to write and diff itself.
    Changed(RewriteKind),
    /// No rewrite was computed.
    Skipped,
    /// `run` produced output identical to the original.
    Unchanged,
}

impl Rewrite {
    /// A `Changed` rewrite for a notebook, carrying the re-emitted
    /// `json` to write and the `before`/`after` code-cell sources the
    /// per-cell diff renders.
    pub(crate) fn notebook(before: Vec<String>, after: Vec<String>, json: String) -> Self {
        Self::Changed(RewriteKind::Notebook(NotebookRewrite {
            after,
            before,
            json,
        }))
    }

    /// A `Changed` rewrite for an ordinary module, written and diffed
    /// as the formatted source.
    pub(crate) fn text(code: String) -> Self {
        Self::Changed(RewriteKind::Text(code))
    }
}

/// The formatted output of a `run`. `Text` writes and diffs its
/// source; `Notebook` writes the re-emitted JSON and diffs each code
/// cell's Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RewriteKind {
    Notebook(NotebookRewrite),
    Text(String),
}

impl RewriteKind {
    /// The content committed on a write: the source for a module, the
    /// re-emitted JSON for a notebook.
    pub(crate) fn written(&self) -> &str {
        match self {
            Self::Notebook(notebook) => &notebook.json,
            Self::Text(code) => code,
        }
    }
}

/// A reformatted notebook: the re-emitted `json` committed on write,
/// and the `before` and `after` code-cell sources the per-cell diff
/// slices.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NotebookRewrite {
    pub after: Vec<String>,
    pub before: Vec<String>,
    pub json: String,
}

//! Plain-data cache records serialized into each entry file.

use std::{path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::diagnostics::Diagnostic;

/// Post-pipeline state cached per `(source, config, version)` key. The
/// diagnostics are always anchored to the source as written, leaving any
/// mode free to render them. The rewrite is present only when the writing
/// mode ran [`Pipeline::run`](crate::pipeline::Pipeline::run).
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

/// What a cached entry holds for the file's rewrite. `Skipped` marks an
/// entry a `check` run wrote without computing the rewrite, whereas the
/// other two record a completed `run`.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rewrite {
    /// `run` produced text differing from the original.
    Changed(String),
    /// No rewrite was computed.
    Skipped,
    /// `run` produced text identical to the original.
    Unchanged,
}

//! Plain-data cache records serialized into each entry file.

use std::{path::PathBuf, time::SystemTime};

use ruff_diagnostics::Fix;
use ruff_notebook::NotebookIndex;
use ruff_text_size::TextRange;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    diagnostics::{Diagnostic, Severity},
    rule::{RuleId, message_for_id},
};

/// Post-pipeline state cached per `(source, config, rules, version)` key
/// under the reading mode's [`Anchor`](crate::cache::Anchor), which
/// names which buffer the diagnostics resolve against, so a `check`
/// entry and a text `format` entry describing one file sit under
/// separate keys rather than overwriting each other. The rewrite is
/// `Skipped` where the reading mode ran no
/// [`Pipeline::run`](crate::pipeline::Pipeline::run).
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CacheEntry {
    #[serde(with = "elided_messages")]
    pub diagnostics: Vec<Diagnostic>,
    pub notebook: Option<NotebookCells>,
    pub rewrite: Rewrite,
}

/// What [`Cache::insert`](crate::cache::Cache::insert) writes, borrowing
/// the outcome the run already holds. Postcard encodes it byte for byte
/// as [`CacheEntry`], so the two share one on-disk shape.
#[derive(Debug, Serialize)]
pub struct CacheEntryRef<'a> {
    #[serde(with = "elided_messages")]
    pub diagnostics: &'a [Diagnostic],
    pub notebook: Option<NotebookCellsRef<'a>>,
    pub rewrite: &'a Rewrite,
}

/// Reads and writes a diagnostic list with each message elided where it
/// matches the static message its rule registers, which is the common
/// case and 11 to 14% of an entry's bytes. Postcard spends one byte on
/// the `Option` discriminant in its place, and the read resolves the
/// elision back through `message_for_id`.
mod elided_messages {
    use super::{
        Deserialize, Deserializer, Diagnostic, Fix, RuleId, Serialize, Serializer, Severity,
        TextRange, message_for_id,
    };

    /// One diagnostic as stored, its `message` `None` where the rule's
    /// own static message would rebuild it.
    #[derive(Deserialize, Serialize)]
    struct Stored<'a> {
        fix: Option<Fix>,
        message: Option<&'a str>,
        range: TextRange,
        rule: RuleId,
        severity: Severity,
    }

    pub(super) fn serialize<S, D>(diagnostics: &D, serializer: S) -> Result<S::Ok, S::Error>
    where
        D: AsRef<[Diagnostic]>,
        S: Serializer,
    {
        serializer.collect_seq(diagnostics.as_ref().iter().map(|d| Stored {
            fix: d.fix.clone(),
            message: (d.message != message_for_id(d.rule)).then_some(d.message.as_str()),
            range: d.range,
            rule: d.rule,
            severity: d.severity,
        }))
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Diagnostic>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<Stored<'_>>::deserialize(deserializer).map(|stored| {
            stored
                .into_iter()
                .map(|s| Diagnostic {
                    fix: s.fix,
                    message: s
                        .message
                        .map_or_else(|| message_for_id(s.rule).to_owned(), ToOwned::to_owned),
                    range: s.range,
                    rule: s.rule,
                    severity: s.severity,
                })
                .collect()
        })
    }
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
    /// Folds another pass's removals into this report.
    pub(super) fn absorb(&mut self, other: Self) {
        self.bytes += other.bytes;
        self.entries += other.entries;
    }

    /// Records one removed file of `bytes`.
    pub(super) fn record(&mut self, bytes: u64) {
        self.bytes += bytes;
        self.entries += 1;
    }
}

/// What a notebook entry carries beyond a module's: the concatenated
/// code its diagnostics resolve against, and the cell index a report
/// renders each position through. A hit reads both back rather than
/// parsing the `.ipynb` JSON a second time. A notebook holding no
/// Python cell carries its own JSON as `code` and no index, which is
/// what the run itself produces for one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NotebookCells {
    pub code: String,
    pub index: Option<NotebookIndex>,
}

/// What an insert writes for a notebook, borrowing the code and index
/// the run holds. Postcard encodes it byte for byte as
/// [`NotebookCells`], so the two share one on-disk shape.
#[derive(Debug, Serialize)]
pub struct NotebookCellsRef<'a> {
    pub code: &'a str,
    pub index: Option<&'a NotebookIndex>,
}

/// What a mode records about the file's rewrite. `Skipped` marks a mode
/// that never computed one, `PassedOver` a file that carries none to
/// make, and the other two a completed `run`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rewrite {
    /// `run` produced output differing from the original, carried as the
    /// kind that writes and diffs itself.
    Changed(RewriteKind),
    /// The file was passed over with no rewrite to make, the shape a
    /// non-Python notebook takes.
    PassedOver,
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

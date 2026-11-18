//! User-level content-addressed cache for `prose check` and `prose format`.
//!
//! Keys are BLAKE3 digests over the source bytes, the file's name, its
//! source type, the active config's canonical TOML, the resolved rule
//! selection, the Prose version, the private `CACHE_FORMAT_VERSION`, and
//! the `Anchor` naming which buffer the diagnostics resolve against,
//! with a notebook's entry also holding the code cells the run read.
//! Entries live one file per key under the platform's cache directory
//! via `PROSE_CACHE_DIR` → `dirs::cache_dir()`, inserts land through a
//! temporary sibling and `rename`, and LRU eviction by mtime sweeps once
//! per run.

mod engine;
mod key;
mod records;

pub use engine::Cache;
pub use key::{Anchor, CacheKey, CacheKeyPrefix};
pub use records::{
    CacheEntry, CacheEntryRef, CacheInfo, CleanReport, NotebookCells, NotebookCellsRef,
    NotebookRewrite, Rewrite, RewriteKind,
};

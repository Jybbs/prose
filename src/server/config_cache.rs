//! Per-document `[tool.prose]` resolution. Each document draws its config
//! from the nearest ancestor project, falling back to its own PEP 723
//! `# /// script` block when no project governs it, then layers any
//! matching per-pattern overrides. With a `didChangeWatchedFiles` watcher
//! registered, each directory's project source is memoized and cleared on
//! a watched change. Without one, resolution re-walks on every call.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use lsp_types::Uri;

use crate::{
    config::{Config, ConfigSource},
    file_uri,
};

/// Resolves the configuration governing each document, memoizing each
/// directory's project source only when a watcher can invalidate the
/// cache on a config change.
#[derive(Default)]
pub(super) struct ConfigCache {
    by_dir: HashMap<PathBuf, DirSource>,
    enabled: bool,
}

impl ConfigCache {
    /// Builds a cache that memoizes only when `enabled`, set by whether a
    /// `didChangeWatchedFiles` watcher was registered.
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }

    /// Drops every memoized source, forcing the next resolve to re-walk
    /// from disk.
    pub(super) fn clear(&mut self) {
        self.by_dir.clear();
    }

    /// Returns the configuration governing `uri`, whose `text` supplies a
    /// standalone script's PEP 723 block when no ancestor project exists.
    /// A watched session memoizes each directory's project source so
    /// sibling documents share one ancestor walk, whereas an unwatched one
    /// re-walks each call. An unsaved buffer whose URI names no file, and a
    /// document under neither a project nor a block, both draw the
    /// defaults.
    pub(super) fn resolve(&mut self, uri: &Uri, text: &str) -> Config {
        let Some(path) = file_uri::to_path(uri) else {
            return Config::default();
        };
        let dir = path.parent().unwrap_or(&path).to_path_buf();
        let config = if self.enabled {
            self.by_dir
                .entry(dir)
                .or_insert_with_key(|dir| DirSource::discover(dir))
                .config(&path, text.as_bytes())
        } else {
            DirSource::discover(&dir).config(&path, text.as_bytes())
        };
        config.unwrap_or_else(Config::default)
    }
}

/// A directory's resolved project source. A bare directory leaves its
/// documents to draw their own script block.
enum DirSource {
    Bare,
    Failed,
    Project(ConfigSource),
}

impl DirSource {
    /// Walks `dir`'s ancestors for a project config, logging a
    /// present-but-broken config before reporting `Failed`.
    fn discover(dir: &Path) -> Self {
        match ConfigSource::discover(dir) {
            Ok(Some(source)) => Self::Project(source),
            Ok(None) => Self::Bare,
            Err(err) => {
                eprintln!(
                    "prose server: config at {} failed to load, using defaults: {err}",
                    dir.display(),
                );
                Self::Failed
            }
        }
    }

    /// The config governing `file`, layering matching overrides onto the
    /// project base or reading `bytes`'s PEP 723 block under a bare
    /// directory. `None` draws the caller back to the defaults, including
    /// when a bare document's block fails to load.
    fn config(&self, file: &Path, bytes: &[u8]) -> Option<Config> {
        match self {
            Self::Bare => match ConfigSource::from_script(file, bytes) {
                Ok(source) => source.map(|source| source.effective_config(file)),
                Err(err) => {
                    eprintln!(
                        "prose server: embedded config in {} failed to load, using defaults: {err}",
                        file.display(),
                    );
                    None
                }
            },
            Self::Failed => None,
            Self::Project(source) => Some(source.effective_config(file)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{uri, write_prose_toml, write_pyproject};

    const SCRIPT: &str = "# /// script\n# [tool.prose]\n# code-line-length = 200\n# ///\nx = 1\n";

    fn doc_uri(path: &Path) -> Uri {
        uri(&file_uri::from_path(&path.display().to_string()))
    }

    fn line_length(config: &Config) -> Option<usize> {
        config.code_line_length.map(std::num::NonZeroUsize::get)
    }

    #[test]
    fn broken_config_logs_and_falls_back_to_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = = oops\n");
        let file = doc_uri(&dir.path().join("mod.py"));

        let mut cache = ConfigCache::new(true);

        assert_eq!(
            line_length(&cache.resolve(&file, "x = 1\n")),
            line_length(&Config::default()),
        );
    }

    #[test]
    fn disabled_cache_re_reads_on_each_resolve() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let file = doc_uri(&dir.path().join("mod.py"));

        let mut cache = ConfigCache::new(false);
        assert_eq!(line_length(&cache.resolve(&file, "x = 1\n")), Some(100));

        write_prose_toml(dir.path(), "code-line-length = 80\n");
        assert_eq!(
            line_length(&cache.resolve(&file, "x = 1\n")),
            Some(80),
            "fresh each call"
        );
    }

    #[test]
    fn enabled_cache_is_stale_until_cleared() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let file = doc_uri(&dir.path().join("mod.py"));

        let mut cache = ConfigCache::new(true);
        assert_eq!(line_length(&cache.resolve(&file, "x = 1\n")), Some(100));

        write_prose_toml(dir.path(), "code-line-length = 80\n");
        assert_eq!(
            line_length(&cache.resolve(&file, "x = 1\n")),
            Some(100),
            "stale until cleared",
        );
        cache.clear();
        assert_eq!(line_length(&cache.resolve(&file, "x = 1\n")), Some(80));
    }

    #[test]
    fn resolve_applies_a_matching_override() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_pyproject(
            dir.path(),
            "[tool.prose]\ncode-line-length = 88\n\n[[tool.prose.overrides]]\npaths = [\"gen/**\"]\ncode-line-length = 200\n",
        );
        let generated = doc_uri(&dir.path().join("gen/x.py"));
        let plain = doc_uri(&dir.path().join("src/x.py"));

        let mut cache = ConfigCache::new(true);

        assert_eq!(
            line_length(&cache.resolve(&generated, "x = 1\n")),
            Some(200)
        );
        assert_eq!(line_length(&cache.resolve(&plain, "x = 1\n")), Some(88));
    }

    #[test]
    fn resolve_falls_back_for_a_bare_document_without_a_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = doc_uri(&dir.path().join("scratch.py"));

        let mut cache = ConfigCache::new(true);

        assert_eq!(
            line_length(&cache.resolve(&file, "x = 1\n")),
            line_length(&Config::default()),
        );
    }

    #[test]
    fn resolve_falls_back_to_default_for_unsaved_buffer() {
        let mut cache = ConfigCache::new(true);
        let resolved = cache.resolve(&uri("untitled:Untitled-1"), "x = 1\n");
        assert_eq!(
            resolved.code_line_length,
            Config::default().code_line_length
        );
    }

    #[test]
    fn resolve_falls_back_when_a_script_block_is_broken() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = doc_uri(&dir.path().join("run.py"));
        let broken = "# /// script\n# [tool.prose\n# ///\nx = 1\n";

        let mut cache = ConfigCache::new(true);

        assert_eq!(
            line_length(&cache.resolve(&file, broken)),
            line_length(&Config::default()),
        );
    }

    #[test]
    fn resolve_ignores_the_block_of_a_project_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_pyproject(dir.path(), "[tool.prose]\ncode-line-length = 88\n");
        let file = doc_uri(&dir.path().join("run.py"));

        let mut cache = ConfigCache::new(true);

        assert_eq!(line_length(&cache.resolve(&file, SCRIPT)), Some(88));
    }

    #[test]
    fn resolve_reads_a_standalone_scripts_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = doc_uri(&dir.path().join("run.py"));

        let mut cache = ConfigCache::new(true);

        assert_eq!(line_length(&cache.resolve(&file, SCRIPT)), Some(200));
    }

    #[test]
    fn resolve_reads_config_for_an_on_disk_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let doc = dir.path().join("mod.py");
        std::fs::write(&doc, "x = 1\n").expect("writes");
        let file = doc_uri(&doc);

        let mut cache = ConfigCache::new(true);

        assert_eq!(line_length(&cache.resolve(&file, "x = 1\n")), Some(100));
    }

    #[test]
    fn resolve_reads_prose_toml_beside_the_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let file = doc_uri(&dir.path().join("mod.py"));

        let mut cache = ConfigCache::new(true);

        assert_eq!(line_length(&cache.resolve(&file, "x = 1\n")), Some(100));
    }

    #[test]
    fn sibling_documents_share_one_resolution() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let first = doc_uri(&dir.path().join("a.py"));
        let second = doc_uri(&dir.path().join("b.py"));

        let mut cache = ConfigCache::new(true);
        assert_eq!(line_length(&cache.resolve(&first, "x = 1\n")), Some(100));

        write_prose_toml(dir.path(), "code-line-length = 80\n");
        assert_eq!(
            line_length(&cache.resolve(&second, "x = 1\n")),
            Some(100),
            "sibling serves the memoized entry",
        );
    }
}

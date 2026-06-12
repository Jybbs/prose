//! Per-document `[tool.prose]` resolution. With a `didChangeWatchedFiles`
//! watcher registered, each directory's config is memoized and cleared on
//! a watched change. Without one, resolution re-reads on every call.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use lsp_types::Uri;

use crate::config::Config;

/// Resolves the configuration governing each document, memoizing per
/// parent directory only when a watcher can invalidate the cache on a
/// config change.
#[derive(Default)]
pub(super) struct ConfigCache {
    by_dir: HashMap<PathBuf, Config>,
    default: Config,
    enabled: bool,
    fresh: Config,
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

    /// Drops every cached config, forcing the next resolve to re-read from
    /// disk.
    pub(super) fn clear(&mut self) {
        self.by_dir.clear();
    }

    /// Returns the configuration governing `uri`. A watched session
    /// memoizes per parent directory so sibling documents share one
    /// resolution, whereas an unwatched one re-reads each call. An
    /// unsaved buffer whose URI names no file falls back to the
    /// defaults.
    pub(super) fn resolve(&mut self, uri: &Uri) -> &Config {
        let Some(path) = file_path(uri) else {
            return &self.default;
        };
        let dir = path.parent().unwrap_or(&path).to_path_buf();
        if self.enabled {
            self.by_dir.entry(dir).or_insert_with_key(|dir| load(dir))
        } else {
            self.fresh = load(&dir);
            &self.fresh
        }
    }
}

/// Turns a `file://` URI into a filesystem path, or `None` for a URI that
/// names no local file.
fn file_path(uri: &Uri) -> Option<PathBuf> {
    if uri.scheme().map(|scheme| scheme.as_str()) != Some("file") {
        return None;
    }
    let decoded = uri.path().as_estr().decode().into_string().ok()?;
    // A Windows drive arrives as `/C:/dir`, so drop its leading slash.
    let path = match decoded.strip_prefix('/') {
        Some(rest) if has_drive_prefix(rest) => rest,
        _ => decoded.as_ref(),
    };
    Some(PathBuf::from(path))
}

/// Returns `true` when `s` opens with a `C:`-style Windows drive letter.
fn has_drive_prefix(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Loads the config governing `path`, logging a present-but-broken config
/// to stderr before falling back to the defaults.
fn load(path: &Path) -> Config {
    Config::load(path).unwrap_or_else(|err| {
        eprintln!(
            "prose server: config at {} failed to load, using defaults: {err}",
            path.display()
        );
        Config::default()
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::testing::write_prose_toml;

    fn line_length(config: &Config) -> Option<usize> {
        config.code_line_length.map(std::num::NonZeroUsize::get)
    }

    fn uri(s: &str) -> Uri {
        Uri::from_str(s).expect("valid uri")
    }

    #[test]
    fn broken_config_logs_and_falls_back_to_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = = oops\n");
        let file = uri(&format!("file://{}", dir.path().join("mod.py").display()));

        let mut cache = ConfigCache::new(true);

        assert_eq!(
            line_length(cache.resolve(&file)),
            line_length(&Config::default()),
        );
    }

    #[test]
    fn disabled_cache_re_reads_on_each_resolve() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let file = uri(&format!("file://{}", dir.path().join("mod.py").display()));

        let mut cache = ConfigCache::new(false);
        assert_eq!(line_length(cache.resolve(&file)), Some(100));

        write_prose_toml(dir.path(), "code-line-length = 80\n");
        assert_eq!(
            line_length(cache.resolve(&file)),
            Some(80),
            "fresh each call"
        );
    }

    #[test]
    fn enabled_cache_is_stale_until_cleared() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let file = uri(&format!("file://{}", dir.path().join("mod.py").display()));

        let mut cache = ConfigCache::new(true);
        assert_eq!(line_length(cache.resolve(&file)), Some(100));

        write_prose_toml(dir.path(), "code-line-length = 80\n");
        assert_eq!(
            line_length(cache.resolve(&file)),
            Some(100),
            "stale until cleared",
        );
        cache.clear();
        assert_eq!(line_length(cache.resolve(&file)), Some(80));
    }

    #[test]
    fn file_path_decodes_percent_escapes() {
        assert_eq!(
            file_path(&uri("file:///tmp/a%20b.py")),
            Some(PathBuf::from("/tmp/a b.py")),
        );
    }

    #[test]
    fn file_path_rejects_a_non_file_scheme() {
        assert!(file_path(&uri("untitled:Untitled-1")).is_none());
    }

    #[test]
    fn file_path_strips_a_windows_drive_slash() {
        assert_eq!(
            file_path(&uri("file:///C:/Users/x.py")),
            Some(PathBuf::from("C:/Users/x.py")),
        );
    }

    #[test]
    fn resolve_falls_back_to_default_for_unsaved_buffer() {
        let mut cache = ConfigCache::new(true);
        let resolved = cache.resolve(&uri("untitled:Untitled-1"));
        assert_eq!(
            resolved.code_line_length,
            Config::default().code_line_length
        );
    }

    #[test]
    fn resolve_reads_config_for_an_on_disk_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let doc = dir.path().join("mod.py");
        std::fs::write(&doc, "x = 1\n").expect("writes");
        let file = uri(&format!("file://{}", doc.display()));

        let mut cache = ConfigCache::new(true);

        assert_eq!(line_length(cache.resolve(&file)), Some(100));
    }

    #[test]
    fn resolve_reads_prose_toml_beside_the_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let file = uri(&format!("file://{}", dir.path().join("mod.py").display()));

        let mut cache = ConfigCache::new(true);

        assert_eq!(line_length(cache.resolve(&file)), Some(100));
    }

    #[test]
    fn sibling_documents_share_one_resolution() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_prose_toml(dir.path(), "code-line-length = 100\n");
        let first = uri(&format!("file://{}", dir.path().join("a.py").display()));
        let second = uri(&format!("file://{}", dir.path().join("b.py").display()));

        let mut cache = ConfigCache::new(true);
        assert_eq!(line_length(cache.resolve(&first)), Some(100));

        write_prose_toml(dir.path(), "code-line-length = 80\n");
        assert_eq!(
            line_length(cache.resolve(&second)),
            Some(100),
            "sibling serves the memoized entry",
        );
    }
}

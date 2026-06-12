//! Per-file config resolution: each input draws its config from its
//! own ancestors, memoized per parent directory.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{config::Config, pipeline::Pipeline, rule::RuleId};

/// Resolves the config governing each input file by walking the
/// file's ancestors, memoizing per parent directory so siblings share
/// one resolution. A directory whose config fails to load reports
/// once and memoizes the failure.
pub(super) struct ConfigResolver {
    by_dir: Mutex<HashMap<PathBuf, Option<Arc<Resolved>>>>,
    ignore: Vec<RuleId>,
    select: Vec<RuleId>,
}

impl ConfigResolver {
    pub(super) fn new(select: Vec<RuleId>, ignore: Vec<RuleId>) -> Self {
        Self {
            by_dir: Mutex::new(HashMap::new()),
            ignore,
            select,
        }
    }

    fn build(&self, config: &Config) -> Resolved {
        Resolved {
            config_toml: toml::to_string(config).unwrap_or_default(),
            pipeline: Pipeline::with_filters(config, &self.select, &self.ignore),
        }
    }

    /// Returns the resolution governing `path`, or `None` when the
    /// walk from its directory finds a config that fails to load.
    pub(super) fn resolve(&self, path: &Path) -> Option<Arc<Resolved>> {
        let file = std::path::absolute(path)
            .inspect_err(|e| eprintln!("error: cannot resolve `{}`: {e}", path.display()))
            .ok()?;
        self.by_dir
            .lock()
            .expect("resolver lock")
            .entry(file.parent().unwrap_or(&file).to_path_buf())
            .or_insert_with_key(|dir| match Config::load(dir) {
                Ok(config) => Some(Arc::new(self.build(&config))),
                Err(e) => {
                    eprintln!("error: loading config for `{}`: {e}", dir.display());
                    None
                }
            })
            .clone()
    }

    /// Pre-resolves `dir` to `config`, so inputs under `dir` reuse it
    /// without re-reading.
    pub(super) fn seed(&self, dir: PathBuf, config: &Config) -> Arc<Resolved> {
        let resolved = Arc::new(self.build(config));
        self.by_dir
            .lock()
            .expect("resolver lock")
            .insert(dir, Some(Arc::clone(&resolved)));
        resolved
    }
}

/// One directory's resolved configuration: the pipeline its enabled
/// rules build and the serialized TOML that keys the cache.
pub(super) struct Resolved {
    pub(super) config_toml: String,
    pub(super) pipeline: Pipeline,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::testing::{assert_send_sync, write_pyproject};

    fn resolver() -> ConfigResolver {
        ConfigResolver::new(Vec::new(), Vec::new())
    }

    #[test]
    fn config_resolver_is_send_and_sync() {
        assert_send_sync::<ConfigResolver>();
    }

    #[test]
    fn resolve_draws_the_files_own_ancestor_config() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");
        let file = tmp.path().join("mod.py");

        let resolved = resolver().resolve(&file).expect("resolves");

        assert!(resolved.config_toml.contains("code-line-length = 120"));
    }

    #[test]
    fn resolve_falls_back_to_defaults_without_an_ancestor_config() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("mod.py");

        let resolved = resolver().resolve(&file).expect("resolves");

        assert!(resolved.config_toml.contains("code-line-length = 88"));
    }

    #[test]
    fn resolve_memoizes_the_failure_of_a_broken_config() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[this is not valid TOML");
        let resolver = resolver();

        assert!(resolver.resolve(&tmp.path().join("a.py")).is_none());
        assert!(resolver.resolve(&tmp.path().join("b.py")).is_none());
    }

    #[test]
    fn resolve_rejects_an_empty_path() {
        assert!(resolver().resolve(Path::new("")).is_none());
    }

    #[test]
    fn resolve_shares_one_resolution_across_siblings() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");
        let resolver = resolver();

        let first = resolver
            .resolve(&tmp.path().join("a.py"))
            .expect("resolves");
        let second = resolver
            .resolve(&tmp.path().join("b.py"))
            .expect("resolves");

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn seed_pre_resolves_the_directory() {
        let tmp = TempDir::new().expect("tempdir");
        let resolver = resolver();

        let seeded = resolver.seed(tmp.path().to_path_buf(), &Config::default());
        let hit = resolver
            .resolve(&tmp.path().join("a.py"))
            .expect("resolves");

        assert!(Arc::ptr_eq(&seeded, &hit));
    }
}

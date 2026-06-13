//! Per-file config resolution: each input draws its effective config
//! from its own ancestors or PEP 723 block, memoizing the per-directory
//! walk and the per-effective-config pipeline so siblings sharing a
//! config build it once.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    config::{Config, ConfigSource},
    pipeline::Pipeline,
    rule::RuleId,
};

/// Resolves the config governing each input file by walking its
/// ancestors for a project config or reading its embedded script block,
/// then layering the overrides its path matches. The per-directory walk
/// and each distinct effective config's pipeline are memoized, while a
/// directory whose config fails to load reports once and fails its files.
pub(super) struct ConfigResolver {
    built: Mutex<HashMap<String, Arc<Resolved>>>,
    default: Arc<Resolved>,
    ignore: Vec<RuleId>,
    select: Vec<RuleId>,
    sources: Mutex<HashMap<PathBuf, DirResolution>>,
}

impl ConfigResolver {
    pub(super) fn new(select: Vec<RuleId>, ignore: Vec<RuleId>) -> Self {
        let default = Arc::new(build_resolved(&Config::default(), &select, &ignore));
        Self {
            built: Mutex::new(HashMap::from([(
                default.config_toml.clone(),
                Arc::clone(&default),
            )])),
            default,
            ignore,
            select,
            sources: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the resolution for an effective `config`, building its
    /// pipeline once and memoizing it under its serialized TOML.
    fn built_for(&self, config: &Config) -> Arc<Resolved> {
        let resolved = Arc::new(build_resolved(config, &self.select, &self.ignore));
        Arc::clone(
            self.built
                .lock()
                .expect("resolver lock")
                .entry(resolved.config_toml.clone())
                .or_insert(resolved),
        )
    }

    /// The resolution governing the directory of `file`, walking its
    /// ancestors once and memoizing the outcome for its siblings.
    fn dir_resolution(&self, file: &Path) -> DirResolution {
        self.sources
            .lock()
            .expect("resolver lock")
            .entry(file.parent().unwrap_or(file).to_path_buf())
            .or_insert_with_key(|dir| match ConfigSource::discover(dir) {
                Ok(Some(source)) => DirResolution::Project(Arc::new(source)),
                Ok(None) => DirResolution::Bare,
                Err(e) => {
                    eprintln!("error: loading config for `{}`: {e}", dir.display());
                    DirResolution::Failed
                }
            })
            .clone()
    }

    /// Returns the resolution for `file` under `source`, reusing a built
    /// pipeline when `file`'s effective config matches one already seen.
    fn resolve_within(&self, source: &ConfigSource, file: &Path) -> Arc<Resolved> {
        let toml = source.effective_toml(file);
        if let Some(resolved) = self.built.lock().expect("resolver lock").get(toml.as_ref()) {
            return Arc::clone(resolved);
        }
        self.built_for(&source.effective_config(file))
    }

    /// Returns the resolution governing `path`, whose `bytes` supply the
    /// script block when no ancestor config exists. `None` when a found
    /// config or embedded block fails to load.
    pub(super) fn resolve(&self, path: &Path, bytes: &[u8]) -> Option<Arc<Resolved>> {
        let file = std::path::absolute(path)
            .inspect_err(|e| eprintln!("error: cannot resolve `{}`: {e}", path.display()))
            .ok()?;
        match self.dir_resolution(&file) {
            DirResolution::Failed => None,
            DirResolution::Project(source) => Some(self.resolve_within(&source, &file)),
            DirResolution::Bare => match ConfigSource::from_script(&file, bytes) {
                Ok(Some(source)) => Some(self.resolve_within(&source, &file)),
                Ok(None) => Some(Arc::clone(&self.default)),
                Err(e) => {
                    eprintln!(
                        "error: loading embedded config for `{}`: {e}",
                        file.display()
                    );
                    None
                }
            },
        }
    }

    /// Builds the resolution for the cwd's own config, governing stdin
    /// and seeding the cache so path inputs resolving to it reuse it.
    pub(super) fn seed(&self, config: &Config) -> Arc<Resolved> {
        self.built_for(config)
    }
}

/// One file's resolved configuration: the pipeline its enabled rules
/// build and the serialized TOML that keys the cache.
pub(super) struct Resolved {
    pub(super) config_toml: String,
    pub(super) pipeline: Pipeline,
}

/// The outcome of walking one directory's ancestors for a project config.
#[derive(Clone)]
enum DirResolution {
    /// No ancestor carried a config, leaving a file here to draw its script block.
    Bare,
    /// A config was found but failed to load, failing its files.
    Failed,
    /// The nearest ancestor config governing files under this directory.
    Project(Arc<ConfigSource>),
}

fn build_resolved(config: &Config, select: &[RuleId], ignore: &[RuleId]) -> Resolved {
    Resolved {
        config_toml: config.to_toml(),
        pipeline: Pipeline::with_filters(config, select, ignore),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::testing::{assert_send_sync, write_pyproject};

    const SCRIPT: &[u8] = b"# /// script\n# [tool.prose]\n# code-line-length = 200\n# ///\nx = 1\n";

    fn resolver() -> ConfigResolver {
        ConfigResolver::new(Vec::new(), Vec::new())
    }

    #[test]
    fn config_resolver_is_send_and_sync() {
        assert_send_sync::<ConfigResolver>();
    }

    #[test]
    fn resolve_applies_a_matching_override() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\ncode-line-length = 88\n\n[[tool.prose.overrides]]\npaths = [\"gen/**\"]\ncode-line-length = 200\n",
        );
        let resolver = resolver();

        let generated = resolver
            .resolve(&tmp.path().join("gen/a.py"), b"x = 1\n")
            .expect("resolves");
        let plain = resolver
            .resolve(&tmp.path().join("src/a.py"), b"x = 1\n")
            .expect("resolves");

        assert!(generated.config_toml.contains("code-line-length = 200"));
        assert!(plain.config_toml.contains("code-line-length = 88"));
    }

    #[test]
    fn resolve_draws_a_standalone_scripts_block() {
        let tmp = TempDir::new().expect("tempdir");

        let resolved = resolver()
            .resolve(&tmp.path().join("run.py"), SCRIPT)
            .expect("resolves");

        assert!(resolved.config_toml.contains("code-line-length = 200"));
    }

    #[test]
    fn resolve_fails_a_standalone_script_with_a_broken_block() {
        let tmp = TempDir::new().expect("tempdir");
        let broken = b"# /// script\n# [tool.prose\n# ///\nx = 1\n";

        assert!(
            resolver()
                .resolve(&tmp.path().join("run.py"), broken)
                .is_none()
        );
    }

    #[test]
    fn resolve_falls_back_to_the_shared_default() {
        let tmp = TempDir::new().expect("tempdir");
        let resolver = resolver();

        let first = resolver
            .resolve(&tmp.path().join("a.py"), b"x = 1\n")
            .expect("resolves");
        let second = resolver
            .resolve(&tmp.path().join("b.py"), b"y = 2\n")
            .expect("resolves");

        assert!(Arc::ptr_eq(&first, &resolver.default));
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn resolve_memoizes_the_failure_of_a_broken_config() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[this is not valid TOML");
        let resolver = resolver();

        assert!(
            resolver
                .resolve(&tmp.path().join("a.py"), b"x = 1\n")
                .is_none()
        );
        assert!(
            resolver
                .resolve(&tmp.path().join("b.py"), b"y = 2\n")
                .is_none()
        );
    }

    #[test]
    fn resolve_project_file_ignores_its_own_script_block() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 88\n");

        let resolved = resolver()
            .resolve(&tmp.path().join("run.py"), SCRIPT)
            .expect("resolves");

        assert!(resolved.config_toml.contains("code-line-length = 88"));
    }

    #[test]
    fn resolve_rejects_an_empty_path() {
        assert!(resolver().resolve(Path::new(""), b"x = 1\n").is_none());
    }

    #[test]
    fn resolve_siblings_under_one_config_share_a_resolution() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");
        let resolver = resolver();

        let first = resolver
            .resolve(&tmp.path().join("a.py"), b"x = 1\n")
            .expect("resolves");
        let second = resolver
            .resolve(&tmp.path().join("b.py"), b"y = 2\n")
            .expect("resolves");

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn resolve_siblings_under_different_overrides_cache_independently() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\ncode-line-length = 88\n\n[[tool.prose.overrides]]\npaths = [\"a.py\"]\ncode-line-length = 200\n",
        );
        let resolver = resolver();

        let matched = resolver
            .resolve(&tmp.path().join("a.py"), b"x = 1\n")
            .expect("resolves");
        let plain = resolver
            .resolve(&tmp.path().join("b.py"), b"y = 2\n")
            .expect("resolves");

        assert!(!Arc::ptr_eq(&matched, &plain));
        assert_ne!(matched.config_toml, plain.config_toml);
    }

    #[test]
    fn seed_resolves_the_cwd_config() {
        let config = Config {
            code_line_length: std::num::NonZeroUsize::new(70),
            ..Config::default()
        };

        let seeded = resolver().seed(&config);

        assert!(seeded.config_toml.contains("code-line-length = 70"));
    }
}

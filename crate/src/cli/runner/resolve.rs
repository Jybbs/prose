//! Per-file config resolution: each input draws its effective config
//! from its own ancestors or PEP 723 block, memoizing the per-directory
//! walk and the per-effective-config pipeline so siblings sharing a
//! config build it once.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rustc_hash::FxHashMap;

use crate::{
    cache::{Anchor, CacheKeyPrefix},
    config::{Config, ConfigSource, DirSource, NoticeDedup, holding_dir},
    pipeline::Pipeline,
    rule::RuleId,
};

/// Resolves the config governing each input file by walking its
/// ancestors for a project config or reading its embedded script block,
/// then layering the overrides its path matches. The per-directory walk
/// and each distinct effective config's pipeline are memoized, while a
/// directory whose config fails to load reports once and fails its files.
pub(super) struct ConfigResolver {
    anchor: Anchor,
    built: Mutex<FxHashMap<String, Arc<Resolved>>>,
    default: Arc<Resolved>,
    ignore: Vec<RuleId>,
    notices: NoticeDedup,
    select: Vec<RuleId>,
    sources: Mutex<FxHashMap<PathBuf, DirSource>>,
}

impl ConfigResolver {
    pub(super) fn new(select: Vec<RuleId>, ignore: Vec<RuleId>, anchor: Anchor) -> Self {
        let resolved = build_resolved(&Config::default(), &select, &ignore, anchor);
        Self::with_default(resolved, anchor, select, ignore)
    }

    /// A resolver answering every bare file with `resolved`.
    #[cfg(test)]
    pub(super) fn over(resolved: Resolved) -> Self {
        Self::with_default(resolved, Anchor::AsWritten, Vec::new(), Vec::new())
    }

    /// A resolver seeded with `resolved` as its default configuration.
    fn with_default(
        resolved: Resolved,
        anchor: Anchor,
        select: Vec<RuleId>,
        ignore: Vec<RuleId>,
    ) -> Self {
        let default = Arc::new(resolved);
        Self {
            anchor,
            built: Mutex::new(FxHashMap::from_iter([(
                default.config_toml.clone(),
                Arc::clone(&default),
            )])),
            default,
            ignore,
            notices: NoticeDedup::default(),
            select,
            sources: Mutex::new(FxHashMap::default()),
        }
    }

    /// Returns the resolution for an effective `config`, building its
    /// pipeline once and memoizing it under its serialized TOML.
    fn built_for(&self, config: &Config) -> Arc<Resolved> {
        let resolved = Arc::new(build_resolved(
            config,
            &self.select,
            &self.ignore,
            self.anchor,
        ));
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
    fn dir_resolution(&self, file: &Path) -> DirSource {
        self.sources
            .lock()
            .expect("resolver lock")
            .entry(holding_dir(file).to_path_buf())
            .or_insert_with_key(|dir| {
                DirSource::discover(dir, &self.notices, |e| {
                    eprintln!("error: loading config for `{}`: {e}", dir.display());
                })
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

    /// The run-scoped notice sink, shared with the cwd config load so
    /// the two loads warn each key once between them.
    pub(super) fn notices(&self) -> &NoticeDedup {
        &self.notices
    }

    /// Returns the resolution governing `path`, whose `bytes` supply the
    /// script block when no ancestor config exists. `None` when a found
    /// config or embedded block fails to load.
    pub(super) fn resolve(&self, path: &Path, bytes: &[u8]) -> Option<Arc<Resolved>> {
        let file = std::path::absolute(path)
            .inspect_err(|e| eprintln!("error: cannot resolve `{}`: {e}", path.display()))
            .ok()?;
        match self.dir_resolution(&file) {
            DirSource::Failed => None,
            DirSource::Project(source) => Some(self.resolve_within(&source, &file)),
            DirSource::Bare => match ConfigSource::from_script(&file, bytes, &self.notices) {
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

/// One file's resolved configuration: the config itself, the pipeline
/// its enabled rules build, the serialized TOML that keys the cache and
/// fills a bug report's configuration field, and the hasher already
/// holding that TOML and rule selection, which every file under this
/// config clones rather than re-absorbing.
pub(super) struct Resolved {
    pub(super) config: Config,
    pub(super) config_toml: String,
    pub(super) key_prefix: CacheKeyPrefix,
    pub(super) pipeline: Pipeline,
}

impl Resolved {
    /// Serializes `config` and loads the key prefix its TOML and
    /// `pipeline`'s selection draw under `anchor`.
    fn new(config: Config, pipeline: Pipeline, anchor: Anchor) -> Self {
        let config_toml = config.to_toml();
        Self {
            key_prefix: CacheKeyPrefix::new(&config_toml, pipeline.rule_ids(), anchor),
            config,
            config_toml,
            pipeline,
        }
    }

    /// A default-config resolution over `pipeline`, the seam a runner
    /// test drives sentinel rules through.
    #[cfg(test)]
    pub(super) fn over(pipeline: Pipeline) -> Self {
        Self::new(Config::default(), pipeline, Anchor::AsWritten)
    }
}

fn build_resolved(
    config: &Config,
    select: &[RuleId],
    ignore: &[RuleId],
    anchor: Anchor,
) -> Resolved {
    Resolved::new(
        config.clone(),
        Pipeline::with_filters(config, select, ignore),
        anchor,
    )
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::testing::{assert_send_sync, write_pyproject};

    const SCRIPT: &[u8] = b"# /// script\n# [tool.prose]\n# code-line-length = 200\n# ///\nx = 1\n";

    fn resolver() -> ConfigResolver {
        ConfigResolver::new(Vec::new(), Vec::new(), Anchor::AsWritten)
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
    fn seed_resolves_the_cwd_config() {
        let config = Config {
            code_line_length: std::num::NonZeroUsize::new(70),
            ..Config::default()
        };

        let seeded = resolver().seed(&config);

        assert!(seeded.config_toml.contains("code-line-length = 70"));
    }
}

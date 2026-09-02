//! A config resolved to its declaring location: the directory globs
//! anchor to, the base `[tool.prose]` table, and its overrides. One
//! `ConfigSource` serves every file under a project, computing each
//! file's effective config by merging the overrides its path matches.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::de::deserialize_prose;
use super::load::{ConfigNotice, NoticeDedup, holding_dir, walk_prose_table};
use super::merge::merge_tables;
use super::overrides::{Override, take_overrides};
use super::{Config, ConfigError, script};

/// The base config and overrides governing files under one directory,
/// alongside the directory their globs anchor to.
pub(crate) struct ConfigSource {
    anchor: PathBuf,
    base: toml::Table,
    base_toml: String,
    overrides: Vec<Override>,
}

impl ConfigSource {
    /// Walks `from`'s ancestors for the nearest `prose.toml`,
    /// `.config/prose.toml`, or `pyproject.toml` carrying `[tool.prose]`,
    /// returning its source or `None` when the chain holds none.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` when a config is found but fails to read,
    /// parse, or compile its overrides.
    pub(crate) fn discover(
        from: &Path,
        notices: &NoticeDedup,
    ) -> Result<Option<Self>, ConfigError> {
        let mut on_notice = |n: ConfigNotice<'_>| notices.emit(n);
        walk_prose_table(from, &mut on_notice)?
            .map(|(anchor, table)| Self::build(anchor, table, &mut on_notice))
            .transpose()
    }

    /// Reads `[tool.prose]` from `bytes`'s leading PEP 723 block as the
    /// base for a standalone `file`, anchoring overrides to the file's own
    /// directory. `None` when the file carries no block.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` when the block is present but malformed.
    pub(crate) fn from_script(
        file: &Path,
        bytes: &[u8],
        notices: &NoticeDedup,
    ) -> Result<Option<Self>, ConfigError> {
        let Some(table) = script::extract_prose_table(bytes)? else {
            return Ok(None);
        };
        let anchor = holding_dir(file).to_path_buf();
        Ok(Some(Self::build(anchor, table, &mut |n| notices.emit(n))?))
    }

    fn build<F>(
        anchor: PathBuf,
        mut table: toml::Table,
        on_notice: &mut F,
    ) -> Result<Self, ConfigError>
    where
        F: FnMut(ConfigNotice<'_>),
    {
        let overrides = take_overrides(&mut table, on_notice)?;
        let base_toml = deserialize_prose(table.clone(), on_notice)?.to_toml();
        Ok(Self {
            anchor,
            base: table,
            base_toml,
            overrides,
        })
    }

    fn deserialize(&self, table: toml::Table) -> Config {
        deserialize_prose(table, &mut |_| {}).expect("base and override bodies validated at load")
    }

    /// The base merged with every override `file` matches, or `None` when
    /// none match.
    fn merged(&self, file: &Path) -> Option<toml::Table> {
        let mut merged: Option<toml::Table> = None;
        for over in self
            .overrides
            .iter()
            .filter(|o| o.matches(file, &self.anchor))
        {
            merge_tables(merged.get_or_insert_with(|| self.base.clone()), over.body());
        }
        merged
    }

    /// The effective `Config` for `file`, deep-merging the body of every
    /// matching override onto the base. Used on a cache miss to build the
    /// pipeline.
    pub(crate) fn effective_config(&self, file: &Path) -> Config {
        self.deserialize(self.merged(file).unwrap_or_else(|| self.base.clone()))
    }

    /// The effective config's serialized TOML, the cache key for `file`.
    /// Borrows the precomputed base when no override matches, sparing the
    /// common case a deserialize round-trip.
    pub(crate) fn effective_toml(&self, file: &Path) -> Cow<'_, str> {
        match self.merged(file) {
            None => Cow::Borrowed(&self.base_toml),
            Some(table) => Cow::Owned(self.deserialize(table).to_toml()),
        }
    }
}

/// The outcome of walking one directory's ancestors for a project
/// config.
#[derive(Clone)]
pub(crate) enum DirSource {
    /// No ancestor carried a config, leaving a file here to draw its
    /// script block.
    Bare,
    /// A config was found but failed to load, failing its files.
    Failed,
    /// The nearest ancestor config governing files under this directory.
    Project(Arc<ConfigSource>),
}

impl DirSource {
    /// Walks `dir`'s ancestors for a project config, handing a
    /// present-but-broken one to `report` before answering `Failed`.
    pub(crate) fn discover(
        dir: &Path,
        notices: &NoticeDedup,
        report: impl FnOnce(ConfigError),
    ) -> Self {
        match ConfigSource::discover(dir, notices) {
            Ok(Some(source)) => Self::Project(Arc::new(source)),
            Ok(None) => Self::Bare,
            Err(err) => {
                report(err);
                Self::Failed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use tempfile::TempDir;

    use super::*;
    use crate::config::MaxShift;
    use crate::testing::{line_length, write_pyproject};

    fn discover(from: &Path) -> Result<Option<ConfigSource>, ConfigError> {
        ConfigSource::discover(from, &NoticeDedup::default())
    }

    #[test]
    fn discover_walks_to_an_ancestor_project() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 120\n");
        let nested = tmp.path().join("pkg/inner");
        fs_err::create_dir_all(&nested).expect("nested dirs create");

        let source = discover(&nested).expect("loads").expect("a source");

        assert_eq!(
            line_length(&source.effective_config(&nested.join("m.py"))),
            Some(120)
        );
    }

    #[test]
    fn discover_without_a_project_yields_none() {
        let tmp = TempDir::new().expect("tempdir");

        assert!(discover(tmp.path()).expect("loads").is_none());
    }

    #[test]
    fn effective_config_applies_a_matching_override() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\ncode-line-length = 88\n\n[[tool.prose.overrides]]\npaths = [\"gen/**\"]\ncode-line-length = 200\n",
        );
        let source = discover(tmp.path()).expect("loads").expect("a source");

        assert_eq!(
            line_length(&source.effective_config(&tmp.path().join("gen/x.py"))),
            Some(200)
        );
        assert_eq!(
            line_length(&source.effective_config(&tmp.path().join("src/x.py"))),
            Some(88)
        );
    }

    #[test]
    fn effective_toml_borrows_base_when_no_override_matches() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(tmp.path(), "[tool.prose]\ncode-line-length = 88\n");
        let source = discover(tmp.path()).expect("loads").expect("a source");

        assert_matches!(
            source.effective_toml(&tmp.path().join("a.py")),
            Cow::Borrowed(_)
        );
    }

    #[test]
    fn override_deep_merges_into_nested_rules() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose.rules]\nalphabetize-siblings = false\n[tool.prose.rules.align-equals]\nmax-shift = 8\n\n[[tool.prose.overrides]]\npaths = [\"wide/**\"]\n[tool.prose.overrides.rules.align-equals]\nmax-shift = 2\n",
        );
        let source = discover(tmp.path()).expect("loads").expect("a source");

        let config = source.effective_config(&tmp.path().join("wide/x.py"));

        assert_eq!(
            config.rules.align_equals.max_shift,
            MaxShift::Cap(std::num::NonZeroUsize::new(2).expect("non-zero")),
        );
        assert!(!config.rules.alphabetize_siblings.enabled);
    }

    #[test]
    fn partial_override_leaves_omitted_knobs_untouched() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\ncode-line-length = 88\ndocstring-line-length = 70\n\n[[tool.prose.overrides]]\npaths = [\"a.py\"]\ncode-line-length = 120\n",
        );
        let source = discover(tmp.path()).expect("loads").expect("a source");

        let config = source.effective_config(&tmp.path().join("a.py"));

        assert_eq!(line_length(&config), Some(120));
        assert_eq!(
            config
                .docstring_line_length
                .map(std::num::NonZeroUsize::get),
            Some(70)
        );
    }

    #[test]
    fn later_matching_override_wins_per_knob() {
        let tmp = TempDir::new().expect("tempdir");
        write_pyproject(
            tmp.path(),
            "[tool.prose]\ncode-line-length = 88\n\n[[tool.prose.overrides]]\npaths = [\"**\"]\ncode-line-length = 100\ndocstring-line-length = 70\n\n[[tool.prose.overrides]]\npaths = [\"a.py\"]\ncode-line-length = 120\n",
        );
        let source = discover(tmp.path()).expect("loads").expect("a source");

        let config = source.effective_config(&tmp.path().join("a.py"));

        assert_eq!(line_length(&config), Some(120));
        assert_eq!(
            config
                .docstring_line_length
                .map(std::num::NonZeroUsize::get),
            Some(70)
        );
    }
}

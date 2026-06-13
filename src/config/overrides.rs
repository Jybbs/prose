//! The `[[tool.prose.overrides]]` array-of-tables: each entry pairs a
//! `paths` glob list with a partial `[tool.prose]` body deep-merged
//! onto a matching file's base config.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, de::IntoDeserializer};

use super::de::deserialize_prose;
use super::load::ConfigNotice;
use super::{Config, ConfigError};

/// One override entry: the glob set its `paths` compile to and the
/// partial body merged over the base of every file the globs match.
#[derive(Debug)]
pub(super) struct Override {
    body: toml::Table,
    paths: GlobSet,
}

impl Override {
    pub(super) fn body(&self) -> &toml::Table {
        &self.body
    }

    /// Whether `file`, taken relative to the declaring config's `anchor`,
    /// matches any of this entry's globs. A `file` outside `anchor` never
    /// matches.
    pub(super) fn matches(&self, file: &Path, anchor: &Path) -> bool {
        file.strip_prefix(anchor)
            .is_ok_and(|relative| self.paths.is_match(relative))
    }
}

/// Captures the required `paths` field, ignoring the body knobs that
/// share the entry table.
#[derive(Deserialize)]
struct OverridePaths {
    paths: Vec<String>,
}

/// Removes `overrides` from `table` and compiles each entry, validating
/// every body against the prose schema. An absent `overrides` key yields
/// an empty list.
///
/// # Errors
///
/// Returns `ConfigError::Toml` when an entry omits `paths` or a body
/// carries an invalid value, and `ConfigError::Glob` when a pattern is
/// not a valid glob.
pub(super) fn take_overrides<F>(
    table: &mut toml::Table,
    on_notice: &mut F,
) -> Result<Vec<Override>, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    let Some(value) = table.remove("overrides") else {
        return Ok(Vec::new());
    };
    Vec::<toml::Table>::deserialize(value.into_deserializer())?
        .into_iter()
        .map(|entry| compile(entry, on_notice))
        .collect()
}

fn compile<F>(mut entry: toml::Table, on_notice: &mut F) -> Result<Override, ConfigError>
where
    F: FnMut(ConfigNotice<'_>),
{
    let OverridePaths { paths } = OverridePaths::deserialize(entry.clone().into_deserializer())?;
    entry.remove("paths");
    let _: Config = deserialize_prose(entry.clone(), on_notice)?;
    Ok(Override {
        body: entry,
        paths: compile_globs(&paths)?,
    })
}

fn compile_globs(patterns: &[String]) -> Result<GlobSet, ConfigError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use assert_matches::assert_matches;

    use super::*;

    fn overrides(toml: &str) -> Result<Vec<Override>, ConfigError> {
        let mut table = toml.parse::<toml::Table>().expect("parses");
        take_overrides(&mut table, &mut |_| {})
    }

    #[test]
    fn absent_overrides_key_yields_empty() {
        assert!(
            overrides("code-line-length = 88\n")
                .expect("parses")
                .is_empty()
        );
    }

    #[test]
    fn body_drops_the_paths_key() {
        let parsed =
            overrides("[[overrides]]\npaths = [\"x\"]\ncode-line-length = 100\n").expect("parses");

        assert!(!parsed[0].body().contains_key("paths"));
        assert_eq!(parsed[0].body()["code-line-length"].as_integer(), Some(100));
    }

    #[test]
    fn entry_without_paths_is_an_error() {
        assert_matches!(
            overrides("[[overrides]]\ncode-line-length = 100\n"),
            Err(ConfigError::Toml(_))
        );
    }

    #[test]
    fn invalid_body_value_is_an_error() {
        assert_matches!(
            overrides("[[overrides]]\npaths = [\"x\"]\ncode-line-length = -1\n"),
            Err(ConfigError::Toml(_))
        );
    }

    #[test]
    fn invalid_glob_is_an_error() {
        assert_matches!(
            overrides("[[overrides]]\npaths = [\"a[\"]\n"),
            Err(ConfigError::Glob(_))
        );
    }

    #[test]
    fn matches_anchors_to_the_declaring_directory() {
        let parsed = overrides("[[overrides]]\npaths = [\"tests/**\"]\n").expect("parses");
        let anchor = Path::new("/proj");

        assert!(parsed[0].matches(Path::new("/proj/tests/unit.py"), anchor));
        assert!(!parsed[0].matches(Path::new("/proj/src/mod.py"), anchor));
    }

    #[test]
    fn matches_any_glob_in_a_multi_pattern_entry() {
        let parsed = overrides("[[overrides]]\npaths = [\"a/**\", \"b/**\"]\n").expect("parses");
        let anchor = Path::new("/proj");

        assert!(parsed[0].matches(Path::new("/proj/b/mod.py"), anchor));
        assert!(!parsed[0].matches(Path::new("/proj/c/mod.py"), anchor));
    }

    #[test]
    fn matches_rejects_a_file_outside_the_anchor() {
        let parsed = overrides("[[overrides]]\npaths = [\"**\"]\n").expect("parses");

        assert!(!parsed[0].matches(Path::new("/other/mod.py"), Path::new("/proj")));
    }
}

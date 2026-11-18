//! The cache key: a BLAKE3 digest over the source, its name and type,
//! the config, the resolved rule selection, and the version inputs,
//! domain-separated by the anchor naming which buffer the entry's
//! diagnostics resolve against.

use ruff_python_ast::PySourceType;

use crate::rules::RuleId;

pub(super) const CACHE_FORMAT_VERSION: &str = "10";

/// How many hex characters of the generation digest name the directory.
const GENERATION_LEN: usize = 16;

/// Which buffer an entry's diagnostics resolve against. A `check` and a
/// structured `format` record `diagnose`'s list against the source as
/// written, whereas a text `format` and a `--diff` run record `run`'s
/// list against the output it rewrote.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Anchor {
    AsWritten,
    Rewritten,
}

impl Anchor {
    /// The BLAKE3 key-derivation context separating this anchor's keys
    /// from the other's. Each string is hardcoded and unique to Prose,
    /// as `Hasher::new_derive_key` requires.
    fn context(self) -> &'static str {
        match self {
            Self::AsWritten => "prose cache entry, diagnostics as written",
            Self::Rewritten => "prose cache entry, diagnostics against the rewrite",
        }
    }
}

/// BLAKE3 digest of the config TOML, rule slugs, Prose version, cache
/// format version, file name, source type, and source bytes, taken under
/// the anchor's key-derivation context. The config TOML, file name,
/// source type, and source bytes are length-framed, the rule slugs are
/// newline-delimited and hold no newline of their own, and the two
/// version strings are fixed by the build.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(pub(super) blake3::Hash);

/// A hasher holding every input a run shares across its files, the
/// anchor's derivation context included, cloned once per file.
#[derive(Clone, Debug)]
pub struct CacheKeyPrefix(blake3::Hasher);

impl CacheKeyPrefix {
    /// Absorbs the config, rule selection, anchor context, and version
    /// strings, the inputs every file in a run shares.
    #[must_use]
    pub fn new(
        config_toml: &str,
        rule_ids: impl IntoIterator<Item = RuleId>,
        anchor: Anchor,
    ) -> Self {
        Self::with_versions(
            config_toml,
            rule_ids,
            anchor,
            env!("CARGO_PKG_VERSION"),
            CACHE_FORMAT_VERSION,
        )
    }

    pub(super) fn with_versions(
        config_toml: &str,
        rule_ids: impl IntoIterator<Item = RuleId>,
        anchor: Anchor,
        prose_version: &str,
        format_version: &str,
    ) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(anchor.context());
        framed(&mut hasher, config_toml.as_bytes());
        for id in rule_ids {
            hasher.update(id.as_str().as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(prose_version.as_bytes());
        hasher.update(format_version.as_bytes());
        Self(hasher)
    }

    /// Completes the digest with one file's own name, source bytes, and
    /// the source type the run reads them as, the type deciding between
    /// the module and the notebook path and the name carrying every
    /// reading a rule takes off the path, `is_package_init` included.
    #[must_use]
    pub fn key_for(&self, name: &str, source_bytes: &[u8], source_type: PySourceType) -> CacheKey {
        let mut hasher = self.0.clone();
        framed(&mut hasher, name.as_bytes());
        framed(
            &mut hasher,
            match source_type {
                PySourceType::Python => b"python",
                PySourceType::Stub => b"stub",
                PySourceType::Ipynb => b"ipynb",
            },
        );
        framed(&mut hasher, source_bytes);
        CacheKey(hasher.finalize())
    }
}

/// The directory segment this build's entries live under. A version
/// bump lands on a fresh segment.
#[must_use]
pub(super) fn generation() -> String {
    generation_for(env!("CARGO_PKG_VERSION"), CACHE_FORMAT_VERSION)
}

/// Absorbs `bytes` behind its little-endian `u64` length.
fn framed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn generation_for(prose_version: &str, format_version: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    framed(&mut hasher, prose_version.as_bytes());
    framed(&mut hasher, format_version.as_bytes());
    hasher.finalize().to_hex()[..GENERATION_LEN].to_owned()
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::HashSet;

    use rstest::rstest;

    use super::*;
    use crate::rules::{align_equals::AlignEquals, alphabetize_siblings::AlphabetizeSiblings};

    pub(in crate::cache) const CONFIG_A: &str = "code-line-length = 88\n";

    const CONFIG_B: &str = "code-line-length = 100\n";

    /// Draws the key `mod.py` takes under `config_toml` and `selection`,
    /// joining the two steps the run path splits across a whole directory.
    pub(in crate::cache) fn key(
        source_bytes: &[u8],
        config_toml: &str,
        selection: impl IntoIterator<Item = RuleId>,
    ) -> CacheKey {
        CacheKeyPrefix::new(config_toml, selection, Anchor::AsWritten).key_for(
            "mod.py",
            source_bytes,
            PySourceType::Python,
        )
    }

    pub(in crate::cache) fn rules() -> [RuleId; 2] {
        [AlignEquals::SLUG, AlphabetizeSiblings::SLUG]
    }

    /// Draws the key `prefix` gives `x = 1` in `mod.py`.
    fn module_key(prefix: &CacheKeyPrefix) -> CacheKey {
        prefix.key_for("mod.py", b"x = 1\n", PySourceType::Python)
    }

    /// Builds the prefix `CONFIG_A` and `rules()` take under the given
    /// versions.
    fn versioned(prose_version: &str, format_version: &str) -> CacheKeyPrefix {
        CacheKeyPrefix::with_versions(
            CONFIG_A,
            rules(),
            Anchor::AsWritten,
            prose_version,
            format_version,
        )
    }

    /// Each pair differs in one input the key reads, the last pair in
    /// selections whose ids would feed the hasher the same `abc` bytes
    /// without the per-id delimiter.
    #[rstest]
    #[case::anchor(
        module_key(&CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::AsWritten)),
        module_key(&CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::Rewritten))
    )]
    #[case::cache_format_version(
        module_key(&versioned(env!("CARGO_PKG_VERSION"), "1")),
        module_key(&versioned(env!("CARGO_PKG_VERSION"), "2"))
    )]
    #[case::config(key(b"x = 1\n", CONFIG_A, rules()), key(b"x = 1\n", CONFIG_B, rules()))]
    #[case::file_name(
        CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::AsWritten).key_for(
            "pkg/__init__.py",
            b"x = 1\n",
            PySourceType::Python
        ),
        key(b"x = 1\n", CONFIG_A, rules())
    )]
    #[case::prose_version(
        module_key(&versioned("0.2.3", CACHE_FORMAT_VERSION)),
        module_key(&versioned("0.3.0", CACHE_FORMAT_VERSION))
    )]
    #[case::rule_selection(
        key(b"x = 1\n", CONFIG_A, [AlignEquals::SLUG]),
        key(b"x = 1\n", CONFIG_A, [AlphabetizeSiblings::SLUG])
    )]
    #[case::source(key(b"x = 1\n", CONFIG_A, rules()), key(b"x = 2\n", CONFIG_A, rules()))]
    #[case::selections_that_concatenate_alike(
        key(b"x = 1\n", CONFIG_A, [RuleId::from("ab"), RuleId::from("c")]),
        key(b"x = 1\n", CONFIG_A, [RuleId::from("a"), RuleId::from("bc")])
    )]
    fn cache_key_differs_when_one_input_changes(#[case] one: CacheKey, #[case] other: CacheKey) {
        assert_ne!(one, other);
    }

    #[test]
    fn cache_key_hex_renders_64_lowercase_chars() {
        let key = key(b"x = 1\n", CONFIG_A, rules());
        let hex = key.0.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn cache_key_is_stable_across_runs() {
        assert_eq!(
            key(b"x = 1\n", CONFIG_A, rules()),
            key(b"x = 1\n", CONFIG_A, rules()),
        );
    }

    #[test]
    fn key_for_separates_every_source_type_of_the_same_bytes() {
        let prefix = CacheKeyPrefix::new(CONFIG_A, rules(), Anchor::AsWritten);
        let keys: HashSet<CacheKey> = [
            PySourceType::Python,
            PySourceType::Stub,
            PySourceType::Ipynb,
        ]
        .map(|source_type| prefix.key_for("mod.py", b"x = 1\n", source_type))
        .into_iter()
        .collect();
        assert_eq!(keys.len(), 3);
    }
}

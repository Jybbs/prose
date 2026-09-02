//! The cache key: a BLAKE3 digest over the source, config, resolved
//! rule selection, and version inputs, domain-separated by the anchor
//! naming which buffer the entry's diagnostics resolve against.

use ruff_python_ast::PySourceType;

use crate::rule::RuleId;

pub(super) const CACHE_FORMAT_VERSION: &str = "9";

/// How many hex characters of the generation digest name the directory.
const GENERATION_LEN: usize = 16;

/// The directory segment this build's entries live under. A version
/// bump lands on a fresh segment, so an earlier build's entries stay
/// out of this build's walk and are reclaimed whole rather than aged
/// out one eviction at a time.
#[must_use]
pub(super) fn generation() -> String {
    generation_for(env!("CARGO_PKG_VERSION"), CACHE_FORMAT_VERSION)
}

fn generation_for(prose_version: &str, format_version: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    framed(&mut hasher, prose_version.as_bytes());
    framed(&mut hasher, format_version.as_bytes());
    hasher.finalize().to_hex()[..GENERATION_LEN].to_owned()
}

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
    /// which is what `Hasher::new_derive_key` documents as the
    /// requirement, and it sets a distinct initial value rather than
    /// mixing a marker into the message, so no arrangement of the
    /// remaining inputs can carry one anchor's key into the other's
    /// space.
    fn context(self) -> &'static str {
        match self {
            Self::AsWritten => "prose cache entry, diagnostics as written",
            Self::Rewritten => "prose cache entry, diagnostics against the rewrite",
        }
    }
}

/// BLAKE3 digest of
/// `config_toml ++ rule_ids ++ prose_version ++ cache_format_version ++ source_bytes`,
/// taken under the anchor's key-derivation context. The config TOML and
/// the source bytes are length-framed so neither can absorb a boundary,
/// the rule slugs are newline-delimited and hold no newline of their
/// own, and the two version strings are fixed by the build rather than
/// by any input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(pub(super) blake3::Hash);

/// A hasher holding every input a run shares across its files, the
/// anchor's derivation context included, cloned per file so the config,
/// rule selection and version tail are absorbed once for the run rather
/// than once for each file.
#[derive(Clone, Debug)]
pub struct CacheKeyPrefix(blake3::Hasher);

impl CacheKeyPrefix {
    /// Loads the run-invariant inputs, so that two runs differing only
    /// in `--select` / `--ignore`, or only in which buffer their
    /// diagnostics resolve against, key separately.
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

    /// Completes the digest with one file's own source bytes and the
    /// source type the run reads them as, which decides between the
    /// module and the notebook path.
    #[must_use]
    pub fn key_for(&self, source_bytes: &[u8], source_type: PySourceType) -> CacheKey {
        let mut hasher = self.0.clone();
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

/// Absorbs `bytes` behind its own length, so a boundary between two
/// variable-length inputs cannot be forged by moving bytes across it.
fn framed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

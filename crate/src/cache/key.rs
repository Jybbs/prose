//! The cache key: a BLAKE3 digest over the source, config, resolved
//! rule selection, and version inputs.

use crate::rule::RuleId;

pub(super) const CACHE_FORMAT_VERSION: &str = "5";

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

/// BLAKE3 digest of
/// `config_toml ++ rule_ids ++ prose_version ++ cache_format_version ++ source_bytes`,
/// each variable-length input length-framed so no pair of inputs can
/// concatenate into another pair's bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(pub(super) blake3::Hash);

/// A hasher holding every input a run shares across its files, cloned
/// per file so the config, rule selection and version tail are absorbed
/// once for the run rather than once for each file.
#[derive(Clone, Debug)]
pub struct CacheKeyPrefix(blake3::Hasher);

impl CacheKeyPrefix {
    /// Loads the run-invariant inputs, so that two runs differing only
    /// in `--select` / `--ignore` key separately.
    #[must_use]
    pub fn new(config_toml: &str, rule_ids: impl IntoIterator<Item = RuleId>) -> Self {
        Self::with_versions(
            config_toml,
            rule_ids,
            env!("CARGO_PKG_VERSION"),
            CACHE_FORMAT_VERSION,
        )
    }

    pub(super) fn with_versions(
        config_toml: &str,
        rule_ids: impl IntoIterator<Item = RuleId>,
        prose_version: &str,
        format_version: &str,
    ) -> Self {
        let mut hasher = blake3::Hasher::new();
        framed(&mut hasher, config_toml.as_bytes());
        for id in rule_ids {
            hasher.update(id.as_str().as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(prose_version.as_bytes());
        hasher.update(format_version.as_bytes());
        Self(hasher)
    }

    /// Completes the digest with one file's own source bytes.
    #[must_use]
    pub fn key_for(&self, source_bytes: &[u8]) -> CacheKey {
        let mut hasher = self.0.clone();
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

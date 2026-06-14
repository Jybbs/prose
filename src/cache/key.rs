//! The cache key: a BLAKE3 digest over the source, config, resolved
//! rule selection, and version inputs.

use crate::rule::RuleId;

pub(super) const CACHE_FORMAT_VERSION: &str = "3";

/// BLAKE3 digest of
/// `source_bytes ++ config_toml ++ rule_ids ++ prose_version ++ cache_format_version`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(pub(super) blake3::Hash);

impl CacheKey {
    /// Computes the key for a source file under the pre-serialized config
    /// TOML and the resolved rule selection that governs it, so two runs
    /// differing only in `--select` / `--ignore` key separately.
    pub fn compute(
        source_bytes: &[u8],
        config_toml: &str,
        rule_ids: impl IntoIterator<Item = RuleId>,
    ) -> Self {
        Self::compute_with_versions(
            source_bytes,
            config_toml,
            rule_ids,
            env!("CARGO_PKG_VERSION"),
            CACHE_FORMAT_VERSION,
        )
    }

    pub(super) fn compute_with_versions(
        source_bytes: &[u8],
        config_toml: &str,
        rule_ids: impl IntoIterator<Item = RuleId>,
        prose_version: &str,
        format_version: &str,
    ) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(source_bytes);
        hasher.update(config_toml.as_bytes());
        for id in rule_ids {
            hasher.update(id.as_str().as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(prose_version.as_bytes());
        hasher.update(format_version.as_bytes());
        Self(hasher.finalize())
    }
}

//! The cache key: a BLAKE3 digest over the source, config, and
//! version inputs.

pub(super) const CACHE_FORMAT_VERSION: &str = "2";

/// BLAKE3 digest of `(source_bytes ++ config_toml ++ prose_version ++ cache_format_version)`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(pub(super) blake3::Hash);

impl CacheKey {
    /// Computes the key for a source file under the pre-serialized config TOML.
    pub fn compute(source_bytes: &[u8], config_toml: &str) -> Self {
        Self::compute_with_versions(
            source_bytes,
            config_toml,
            env!("CARGO_PKG_VERSION"),
            CACHE_FORMAT_VERSION,
        )
    }

    pub(super) fn compute_with_versions(
        source_bytes: &[u8],
        config_toml: &str,
        prose_version: &str,
        format_version: &str,
    ) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(source_bytes);
        hasher.update(config_toml.as_bytes());
        hasher.update(prose_version.as_bytes());
        hasher.update(format_version.as_bytes());
        Self(hasher.finalize())
    }
}

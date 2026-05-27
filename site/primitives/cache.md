# Cache

<PrimitiveLayout primitive="cache">

*Cache* is the user-level content-addressed cache that lets `prose check` and `prose format` skip the pipeline for unchanged source. Each entry is a bincode-serialized payload carrying the post-pipeline diagnostics and optional rewrite, keyed on the **BLAKE3** digest of `(source_bytes ++ config_toml ++ prose_version ++ cache_format_version)`. A repeat run against an unchanged file collapses to a stat plus a hash plus a deserialize, with no AST construction, no rule pipeline, and no rewrite computation.

## Consumer-Visible Surface

*Cache* lives at `src/cache.rs` and is `pub(crate)`, so the type is documented here for the consumer-visible CLI behavior it shapes rather than as a directly-callable type. The downstream-visible consequences are the `prose cache` subcommands *(`clean`, `compact`, `info`)*, the `--no-cache` flag on `prose check` and `prose format`, the `--verbose` flag's hit/miss telemetry, and the `[tool.prose.cache]` configuration table. The [**Cache**](/reference/cache) reference covers each surface from a user's perspective.

A downstream consumer in `0.2.x` reaches the cache indirectly through `cli::runner::process_path`. Each file's bytes feed `CacheKey::compute`, the resulting key drives a lookup, and on hit the runner rehydrates the cached diagnostics and rewrite into a `SourceFile` without entering the pipeline. On miss, the runner runs the pipeline as normal and inserts the resulting entry before emitting.

At `1.0` the cache surface stabilizes for downstream consumers integrating the pipeline directly.

## Key Shape

The cache key is the **BLAKE3** digest of inputs concatenated in order: the source bytes, the canonical TOML serialization of the active `Config`, the *Prose* version from `CARGO_PKG_VERSION`, and a private `CACHE_FORMAT_VERSION` constant.

A change to any one input produces a different key, so a config tweak invalidates only the entries it semantically affects, and a *Prose* release invalidates the entire cache. The `CACHE_FORMAT_VERSION` input lets the on-disk entry shape bump independently of the user-facing version, leaving a release that does not change the entry shape free to carry its existing cache forward.

The canonical TOML serialization runs through `toml::to_string`, so a semantically-equivalent re-shuffling of the user's `pyproject.toml` produces the same key. Two workspaces editing identical files under matching configuration share a cache hit, because the key already disambiguates source content across projects.

## LRU Eviction

`Cache::insert` runs a best-effort LRU pass after every successful write. The pass collects every entry's last-access mtime, sorts ascending, and removes entries until the directory total falls back under the configured cap *(default 100 MiB)*. Permission failures and concurrent-eviction races log to stderr as warnings and never block the insert.

`Cache::lookup` bumps the entry's mtime on hit, so the LRU sweep keeps recently-accessed entries even when they sit older in absolute terms. `Cache::compact` exposes the same eviction pass as an on-demand operation, useful after lowering `max-size-mib` so the new ceiling lands without waiting for the next insert.

## Atomic Writes

`Cache::insert` writes the bincode payload to a `<key>.<pid>.tmp` sidecar and renames it onto the final `<key>` path, so the rename's POSIX atomicity guarantees a concurrent reader never observes a partial entry. The sidecar is cleaned up on rename failure, and `Cache::info` filters `.tmp` files out of its directory walk via `path.extension().is_none()`.

## Path Resolution

Resolution chains through `PROSE_CACHE_DIR` → `XDG_CACHE_HOME/prose` → `dirs::cache_dir().join("prose")`. `PROSE_CACHE_DIR` is taken as-is with no subdirectory appended, so a CI runner or test harness pins the cache to a known path independent of the runner's HOME layout. `XDG_CACHE_HOME` is honored on every platform rather than only on Linux.

## Re-Using This Primitive

The cache is consumed by the CLI's `prose check` and `prose format` entry points and the three `prose cache` subcommands. A downstream Rust consumer integrating *Prose* through `Pipeline::run` typically holds its own caching layer above or below the pipeline, since the per-file cache hit semantics depend on the consumer's build-system or editor lifecycle rather than on the user-level cache directory.

<template #related>

- [[source]] is the value the cache shortcuts: a hit produces a `SourceFile` from the cached payload without re-parsing.
- [[edit]] is the rewrite shape every cached entry carries alongside its diagnostics.
- The [**Cache**](/reference/cache) reference covers the cache from a user's perspective.

</template>

</PrimitiveLayout>

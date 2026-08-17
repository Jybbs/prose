---
consumedBy: [cli]
consumes: [source]
layer: analysis
stability: internal
summary: "User-level on-disk cache keyed on `(source ++ config ++ rules ++ version)` under a per-anchor derivation context, collapsing repeat runs to a stat plus a hash plus a deserialize."
tagline: content-addressed result cache
---

# Cache

<PrimitiveLayout primitive="cache">

*Cache* is the user-level content-addressed cache that lets `prose check` and `prose format` skip the pipeline for unchanged source. Each entry is a `postcard`-serialized payload carrying the post-pipeline diagnostics and optional rewrite, keyed on the **BLAKE3** digest of `(config_toml ++ rule_ids ++ prose_version ++ cache_format_version ++ source_bytes)` taken under a per-anchor derivation context. A repeat run against an unchanged file collapses to a stat plus a hash plus a deserialize, with no AST construction, no rule pipeline, and no rewrite computation.

## Consumer-Visible Surface

*Cache* lives at `crate/src/cache/` and is `pub(crate)`, so the type is documented here for the consumer-visible CLI behavior it shapes rather than as a directly-callable type. The downstream-visible consequences are the `prose cache` subcommands *(`clean`, `compact`, `info`)*, the `--no-cache` flag on `prose check` and `prose format`, the `--verbose` flag's hit/miss telemetry, and the `[cache]` configuration table. The [**Cache**](/reference/cache) reference covers each surface from a user's perspective.

A downstream consumer reaches the cache indirectly through `cli::runner::process_path`. Each file's bytes complete the run's shared `CacheKeyPrefix` through `key_for`, the resulting key drives a lookup, and on hit the runner rehydrates the cached diagnostics and rewrite into a `SourceFile` without entering the pipeline. On miss, the runner runs the pipeline as normal and inserts the resulting entry where a later run could still reach it.

What an entry carries tracks the mode that wrote it, and the key's anchor input keeps the two kinds apart. A `check` and a structured `format` record `diagnose`'s diagnostics against the source as written, whereas plain `format` and `format --diff` record `run`'s diagnostics against the output it rewrote, so neither mode is ever served an entry holding the other's list. Within the as-written anchor a `check` marks the rewrite skipped, since it reads no rewritten text, leaving a later structured `format` on that entry to recompute the rewrite it needs rather than trust an absent one. `check --validate` bypasses the cache outright, because it re-confirms each rewrite parses rather than trust an entry an earlier run left unvalidated. A write-back `format` stores nothing for a file it rewrites, the commit replacing the bytes the key was drawn from so that file never reads the entry back. Beside the diagnostics and the rewrite an entry carries the settle report where the run that wrote it built one, so a hit re-renders the unstable-output notice rather than dropping it.

At `1.0` the cache surface stabilizes for downstream consumers integrating the pipeline directly.

## Key Shape

The cache key is the **BLAKE3** digest of inputs concatenated in order: the canonical TOML serialization of the active `Config`, the resolved rule selection the pipeline runs, the *Prose* version from `CARGO_PKG_VERSION`, a private `CACHE_FORMAT_VERSION` constant, and the file's own source bytes. The anchor naming which buffer the entry's diagnostics resolve against enters ahead of all of them, as the `Hasher::new_derive_key` context the digest opens under, so no arrangement of the remaining inputs can carry one anchor's key into the other's space.

A change to any one input produces a different key, so a config tweak invalidates only the entries it semantically affects, a `--select` or `--ignore` run keys apart from a full one, and a *Prose* release invalidates the entire cache. The `CACHE_FORMAT_VERSION` input lets the on-disk entry shape bump independently of the user-facing version, leaving a release that does not change the entry shape free to carry its existing cache forward.

The canonical TOML serialization runs through `toml::to_string`, so a semantically-equivalent re-shuffling of the user's config file produces the same key. Two workspaces editing identical files under matching configuration share a cache hit, because the key already disambiguates source content across projects.

## LRU Eviction

A best-effort LRU pass runs once a path run's inserts have landed, called from `cli::runner`'s `RunSetup::walked` rather than from `Cache::insert`. The pass reclaims any generation directory an older build left behind, then where the live generation still sits over either configured cap *(defaulting to 100 MiB and 10,000 entries)* it collects every entry's last-access mtime, sorts ascending, and removes entries until both totals sit at four fifths of their cap, so the next run's inserts land inside the ceiling instead of paying for another sort. `Cache::insert` records that a write landed, and a run that only read entries skips the sweep on that flag. Permission failures and concurrent-eviction races log to stderr as warnings and never block an insert.

`Cache::lookup` bumps the entry's mtime on hit, so the LRU sweep keeps recently-accessed entries even when they sit older in absolute terms. `Cache::compact` is that pass, which `prose cache compact` also exposes as an on-demand operation, useful after lowering `max-size-mib` so the new ceiling lands without waiting for the next run.

## Atomic Writes

`Cache::insert` writes the `postcard` payload to a `tempfile`-managed sibling with a `.tmp` suffix and renames it onto the final `<key>` path, so the rename's POSIX atomicity guarantees a concurrent reader never observes a partial entry. The sibling is cleaned up on drop when the rename fails, and `Cache::info` filters `.tmp` files out of its directory walk via `path.extension().is_none()`.

## Path Resolution

Resolution chains through `PROSE_CACHE_DIR` → `dirs::cache_dir().join("prose")`. `PROSE_CACHE_DIR` is taken as-is with no subdirectory appended, so a CI runner or test harness pins the cache to a known path independent of the runner's HOME layout. The `dirs` crate already honors `XDG_CACHE_HOME` on Linux, so the Linux default still respects the XDG variable when set.

## Re-Using This Primitive

The cache is consumed by the CLI's `prose check` and `prose format` entry points and every `prose cache` subcommand. A downstream Rust consumer integrating *Prose* through `Pipeline::run` typically holds its own caching layer above or below the pipeline, since the per-file cache hit semantics depend on the consumer's build-system or editor lifecycle rather than on the user-level cache directory.

<template #related>

- [[source]] is the value the cache shortcuts, in that a hit produces a `SourceFile` from the cached payload without re-parsing.
- [[edit]] is the rewrite shape every cached entry carries alongside its diagnostics.
- The [**Cache**](/reference/cache) reference covers the cache from a user's perspective.

</template>

</PrimitiveLayout>

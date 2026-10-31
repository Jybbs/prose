---
consumedBy: [cli]
consumes: [source]
layer: analysis
stability: internal
summary: "Stores each file's diagnostics and rewrite on disk under a key derived from its source, config, rules, and version, so a repeat run over an unchanged file skips the pipeline."
tagline: content-addressed result cache
---

# The `Cache` Type

<PrimitiveLayout primitive="cache">

*Cache* is the user-level content-addressed cache that lets `prose check` and `prose format` skip the pipeline for unchanged source. Each entry is a `postcard`-serialized payload carrying the post-pipeline diagnostics and optional rewrite, under the key [**Key Shape**](#key-shape) sets out. A repeat run over an unchanged file costs a stat, a hash, and a deserialize, with no AST construction, no rule pipeline, and no rewrite computation.

## Consumer-Visible Surface

*Cache* lives at `crate/src/cache/` and is `pub(crate)`, so this page documents it for the CLI behavior it produces rather than as a type a downstream calls directly. What a downstream sees is the `prose cache` subcommands (*`clean`, `compact`, `info`*), the `--no-cache` flag on `prose check` and `prose format`, the hit and miss counts `--verbose` prints, and the `[cache]` configuration table. The [**Cache**](/reference/cache) reference covers each of them from a user's perspective.

The CLI reaches the cache through `cli::runner::process_path`. Each file's path, bytes, and source type complete the run's shared `CacheKeyPrefix` through `key_for`, the resulting key drives a lookup, and on a hit the runner rebuilds a `SourceFile` from the cached diagnostics and rewrite without entering the pipeline. On a miss, the runner runs the pipeline as normal and inserts the resulting entry where a later run can find it.

What an entry carries depends on the mode that wrote it. A `check` and a structured `format` record `diagnose`'s diagnostics against the source as written, whereas a plain `format` and a `format --diff` record `run`'s diagnostics against the output it rewrote. The key's anchor input keeps the two kinds apart, so neither mode is ever served an entry carrying the other's list. Within the as-written anchor a `check` marks the rewrite skipped, since it reads no rewritten text, so a later structured `format` on that entry recomputes the rewrite it needs rather than reading an absent one. `check --validate` bypasses the cache outright, because it re-confirms that each rewrite parses, compiles, and settles, which an entry an earlier run wrote unvalidated cannot show. A write-back `format` stores nothing for a file it rewrites, in that the commit replaces the bytes the key was derived from, so that file never reads the entry back. Beside the diagnostics and the rewrite an entry carries the settle report where the run that wrote it built one, so a hit prints the unstable-output notice again rather than dropping it.

## Key Shape

The cache key is the **BLAKE3** digest of `(config_toml ++ rule_ids ++ prose_version ++ cache_format_version ++ name ++ source_type ++ source_bytes)`, these inputs concatenated in order:

1. The canonical TOML serialization of the active `Config`.
2. The resolved rule selection the pipeline runs.
3. The *Prose* version from `CARGO_PKG_VERSION`.
4. A private `CACHE_FORMAT_VERSION` constant.
5. The file's name, as the run read its path, since a rule reads the path (*an `__init__.py` marks a package*), so two files holding the same bytes key apart.
6. The source type the run reads the file as, `python`, `stub`, or `ipynb`, so a module and a notebook holding the same bytes key apart.
7. The file's own source bytes.

The anchor enters ahead of all of them, as the `Hasher::new_derive_key` context the digest opens under. It names which buffer the entry's diagnostics resolve against, so no arrangement of the remaining inputs can carry one anchor's key into the other's space.

A change to any one input produces a different key, so a config edit invalidates only the entries whose settings it changes, a `--select` or `--ignore` run keys apart from a full one, and a *Prose* release invalidates the entire cache. The `CACHE_FORMAT_VERSION` input lets the on-disk entry format change independently of the user-facing version, so the entry format can change without a version bump, and either change starts a new generation.

The canonical TOML serialization runs through `toml::to_string`, so reordering the keys in a config file without changing their values produces the same key. Two working copies holding identical files under matching configuration share a cache hit only where each run names a file by the same path, as `prose check .` run from each copy's root does, because the name enters the key beside the bytes and an absolute path names each copy apart.

## LRU Eviction

A best-effort LRU pass runs once a path run's inserts have been written, called at the end of `cli::runner`'s `RunSetup::walked` and `RunSetup::streamed` rather than from `Cache::insert`. The pass first deletes every other build's generation directory that has sat untouched for an hour. It then reads every entry's metadata to total the live generation, and where that total exceeds either configured cap (*defaulting to <ConfigDefault section="cache" facet="max-size-mib" /> MiB and <ConfigDefault section="cache" facet="max-entries" /> entries*), sorts the entries by last-access mtime and removes the oldest until both totals sit at four fifths of their cap, so the next run's inserts fit inside the ceiling instead of paying for another sort. Sweeping once per run rather than once per insert reads each entry's metadata once, however many entries the run inserts. `Cache::insert` records that a write succeeded, and a run that only read entries skips the sweep on that flag. Permission failures and concurrent-eviction races are logged to stderr as warnings and never block an insert.

`Cache::lookup` bumps the entry's mtime on a hit whose recorded mtime is more than an hour old, so the LRU sweep keeps a recently read entry even when it is older in absolute terms. `Cache::compact` is the LRU pass itself, which `prose cache compact` also exposes as an on-demand operation.

## Atomic Writes

`Cache::insert` writes the `postcard` payload to a `tempfile`-managed sibling with a `.tmp` suffix and renames it onto the final `<key>` path, so a concurrent reader never sees a partial entry, because a POSIX rename is atomic. The sibling is deleted on drop when the rename fails, `Cache::info` filters `.tmp` files out of the directory listing it reads through `path.extension().is_none()`, and `prose cache clean` deletes any `.tmp` file an interrupted insert left.

## Path Resolution

The cache path resolves through `PROSE_CACHE_DIR` first and `dirs::cache_dir().join("prose")` otherwise, and the [**Location**](/reference/cache#location) section of the Cache reference lists the path that resolves to on each platform.

## Re-Using This Primitive

A downstream Rust consumer integrating *Prose* through `Pipeline::run` typically keeps its own caching layer above or below the pipeline, since the validity of a per-file cache hit depends on the consumer's build-system or editor lifecycle rather than on the user-level cache directory.

<template #related>

- [[source]] is the value the cache skips, in that a hit produces a `SourceFile` from the cached payload without re-parsing.
- [[edit]] is the rewrite type every cached entry carries alongside its diagnostics.
- The [**Cache**](/reference/cache) reference covers the cache from a user's perspective.

</template>

</PrimitiveLayout>

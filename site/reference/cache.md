---
description: "Covers the per-user cache, the `[cache]` keys, the `--no-cache` flag, and the `prose cache` subcommands."
---

# Cache

*Prose* caches each file's result, keyed on the file's bytes, its path as the run names it, the source type the run reads it as, the configuration that governs it, the rules the run selects, the *Prose* version, and the diagnostic anchor, meaning whether the entry's diagnostics refer to the text before or after a rewrite. A repeat `prose check` or `prose format` over an unchanged file then costs a stat, a hash, and a deserialize, because a cache hit prints the stored diagnostics without running the pipeline. The [**`Cache` Type**](/architecture/cache) page covers how the key is derived from those inputs and how eviction keeps the cache within its caps.

The cache is on by default, with the `[cache]` table tuning it, `--no-cache` bypassing it for one run, and `prose cache clean` emptying it.

A `prose format` run that rewrites a file stores no entry for it, because the rewrite replaces the bytes the key was computed from, so that file's next run computes a new key. A file the run leaves unchanged does get an entry, which is why a second `prose format` over an already-formatted tree hits on every file. The rewriting run records the key of the bytes it wrote in the generation's ledger, so a later `prose format` run that finds those bytes changing again raises the [**unstable-output**](/reference/cli#unstable-output) notice.

## Location

The cache lives per user, at a path that depends on the platform:

| **Platform** | **Path** |
|---|---|
| Linux | `$XDG_CACHE_HOME/prose` (*default `~/.cache/prose`*) |
| macOS | `~/Library/Caches/prose` |
| Windows | `%LOCALAPPDATA%\prose` |

Each entry is one file named by the 64-character lowercase hex form of its key, stored in `postcard` for small size and fast deserialization, inside a generation subdirectory named by a digest of the *Prose* version and the private entry-format version. One `ls` a level down shows the layout. Grouping entries by generation lets a version bump delete its predecessor's entries in one step rather than evicting them one at a time, and the older directory is kept for an hour before that deletion in case a run of the older build is still reading it.

`PROSE_CACHE_DIR` overrides the platform default and is used exactly as given, with no subdirectory appended, so a CI runner or a test harness can pin the cache to a known path whatever the runner's home directory looks like. The platform default comes from the [`dirs`](https://docs.rs/dirs) crate, which reads `XDG_CACHE_HOME` on Linux.

## Configuration

The keys under the `[cache]` table (*`[tool.prose.cache]` in a `pyproject.toml`*):

<ConfigKeys section="cache" />

The sample below raises both caps above their defaults, for a tree whose entries outgrow them:

```toml
[cache]
enabled      = true
max-entries  = 25000
max-size-mib = 250
```

## `--no-cache`

`--no-cache` skips both lookups and writes for one run of `prose check` or `prose format`, overriding a configured `enabled = true`.

```bash
prose check --no-cache .
prose format --no-cache src/
```

## `prose cache clean`

`prose cache clean` deletes every entry, along with any `.tmp` file an interrupted insert left, and prints the bytes freed and the number of entries deleted.

```bash
$ prose cache clean
removed 142 entries (8124416 bytes)
```

It exits `0` on success and `4` on a permission or filesystem failure, with the error printed to stderr.

## `prose cache compact`

`prose cache compact` runs the eviction pass immediately, evicting entries from a cache past its `[cache] max-size-mib` or `max-entries` cap and printing the bytes and entries it removed. Eviction otherwise runs only at the end of a run over paths that wrote an entry, so a project that lowers a cap sees the new cap take effect only after the next such run, or after `compact`.

```bash
$ prose cache compact
removed 17 entries (2097152 bytes)
```

## `prose cache info`

`prose cache info` prints the cache directory's resolved path, its entry count, its total size in bytes, and the mtimes of its oldest and newest entries as relative ages. It shows whether `PROSE_CACHE_DIR` resolved to the expected path and whether recent runs are writing entries.

```bash
$ prose cache info
path: ~/Library/Caches/prose
entries: 142
bytes: 8124416
oldest: 2d ago
newest: 5m ago
```

## Hit-Miss Telemetry

The global `--verbose` flag prints one line of cache counts to stderr at the end of each `prose check` or `prose format` run:

```bash
$ prose --verbose check src/
cache: 23 hits, 4 misses, 27 files
```

When `--no-cache` is set or `[cache] enabled = false`, the line reads `cache: bypassed`.

## Corrupt-Entry Recovery

A read or write error never fails a run. A corrupt entry counts as a miss, the run formats the file in full, and the next insert overwrites the corrupt entry.

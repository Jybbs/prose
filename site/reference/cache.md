# Cache

*Prose* caches per-file pipeline output keyed on the source bytes, the active configuration, and the *Prose* version. A repeat `prose check` or `prose format` against an unchanged file collapses to a stat, a hash, and a deserialize, since the cache hit re-emits diagnostics from the cached entry without entering the pipeline.

The cache is enabled by default. The `[cache]` table tunes it, `--no-cache` bypasses it for one invocation, and `prose cache clean` clears it.

## Location

The cache lives at the user level, with the path resolving per platform:

| Platform | Path |
|---|---|
| Linux | `$XDG_CACHE_HOME/prose` *(default `~/.cache/prose`)* |
| macOS | `~/Library/Caches/prose` |
| Windows | `%LOCALAPPDATA%\prose\cache` |

Each cache entry is one file under that directory, named by the 64-character lowercase hex form of the entry's key. The layout is flat and inspectable with `ls`, and the on-disk format is bincode for compact size and fast deserialize.

Resolution chains through `PROSE_CACHE_DIR` → the platform default. `PROSE_CACHE_DIR` is taken as-is with no subdirectory appended, so a CI runner or test harness pins the cache to a known path independent of the runner's HOME layout. The platform default flows through the [`dirs`](https://docs.rs/dirs) crate, which already honors `XDG_CACHE_HOME` on Linux.

## Key Shape

The cache key is the **BLAKE3** digest of `(source_bytes ++ config_toml ++ prose_version ++ cache_format_version)`. The inputs:

- the source bytes of the file under formatting
- the canonical TOML serialization of the active `Config`, so a semantically-equivalent re-shuffling of the user's config file produces the same key
- the *Prose* version from `CARGO_PKG_VERSION`, so a version bump invalidates the cache wholesale
- a private `CACHE_FORMAT_VERSION` constant that bumps independently when the on-disk entry shape changes, leaving the user-facing version free to ship semver-meaningful bumps without unrelated cache turnover

Two workspaces editing identical files under matching configuration share a cache hit, because the key already disambiguates source content across projects.

## Eviction

LRU eviction runs on every insert. The pass collects every entry's last-access mtime, sorts ascending, and removes entries until the directory total falls back under the configured cap *(default 100 MiB)*. The pass is best-effort and never blocks the insert, with permission failures and concurrent-eviction races logged to stderr as warnings.

Inserts write to a `.tmp`-suffixed sibling then `rename` onto the final path, so the rename's POSIX atomicity guarantees a concurrent reader never observes a partial entry. The sibling is cleaned up on drop when the rename fails, and `prose cache clean` sweeps any orphaned `.tmp` files alongside cache entries.

## Configuration

The knobs under the `[cache]` table *(`[tool.prose.cache]` in a `pyproject.toml`)*:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the cache globally |
| `max-size-mib` | positive int | `100` | LRU eviction cap on the cache directory |

```toml
[cache]
enabled      = true
max-size-mib = 250
```

## `--no-cache`

The `--no-cache` flag bypasses both lookups and writes for a single invocation, on either `prose check` or `prose format`. The flag overrides the configured `enabled` value.

```bash
prose check --no-cache .
prose format --no-cache src/
```

## `prose cache clean`

The `prose cache clean` subcommand removes every cache entry and prints the freed byte count plus the cleared entry count.

```bash
$ prose cache clean
removed 142 entries (8124416 bytes)
```

Returns exit code 0 on success. The IO-error exit code applies on permission or filesystem failures.

## `prose cache compact`

The `prose cache compact` subcommand runs the LRU eviction pass against the cache, reducing it to the configured `[cache] max-size-mib` cap. Eviction normally runs only on insert, so a project that lowered its cap will not see the new ceiling enforced until the next `prose check` or `prose format` writes a fresh entry. `compact` triggers eviction immediately and reports the bytes and entry count it removed.

```bash
$ prose cache compact
removed 17 entries (2097152 bytes)
```

## `prose cache info`

The `prose cache info` subcommand reports the cache directory's resolved path, total entry count, total byte size, and the oldest and newest entry mtimes *(as relative ages)*. Useful for verifying that `PROSE_CACHE_DIR` resolved where expected, or that the cache is being populated by recent runs.

```bash
$ prose cache info
path: /Users/jybbs/Library/Caches/prose
entries: 142
bytes: 8124416
oldest: 2d ago
newest: 5m ago
```

## Hit-Miss Telemetry

The global `--verbose` flag prints a one-line cache summary to stderr at the end of each `prose check` or `prose format` run:

```bash
$ prose --verbose check src/
cache: 23 hits, 4 misses, 27 files
```

When `--no-cache` is set or `[cache] enabled = false`, the line reads `cache: bypassed` to surface that the cache was skipped for the run.

## Corrupt-Entry Recovery

The cache silently degrades on read or write errors. A corrupt entry produces a cache miss, the runner falls through to a full pipeline run, and the corrupt entry is overwritten on the subsequent insert.

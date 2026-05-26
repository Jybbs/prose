# Cache

*Prose* caches per-file pipeline output keyed on the source bytes, the active configuration, and the *Prose* version. A repeat `prose check` or `prose format` against an unchanged file collapses to a stat, a hash, and a deserialize, since the cache hit re-emits diagnostics from the cached entry without entering the pipeline.

The cache is enabled by default. The `[tool.prose.cache]` sub-table tunes it, `--no-cache` bypasses it for one invocation, and `prose cache clean` clears it.

## Location

The cache lives at the user level, with the path resolving per platform:

| Platform | Path |
|---|---|
| Linux | `$XDG_CACHE_HOME/prose` *(default `~/.cache/prose`)* |
| macOS | `~/Library/Caches/prose` |
| Windows | `%LOCALAPPDATA%\prose\cache` |

Each cache entry is one file under that directory, named by the 64-character lowercase hex form of the entry's key. The layout is flat and inspectable with `ls`, and the on-disk format is bincode for compact size and fast deserialize.

## Key Shape

The cache key is the **BLAKE3** digest of `(source_bytes ++ config_toml ++ prose_version)`. Three inputs feed the hash:

- the source bytes of the file under formatting
- the canonical TOML serialization of the active `Config`, so a semantically-equivalent re-shuffling of the user's `pyproject.toml` produces the same key
- the *Prose* version from `CARGO_PKG_VERSION`, so a version bump invalidates the cache wholesale

Two workspaces editing identical files under matching configuration share a cache hit, because the key already disambiguates source content across projects.

## Eviction

LRU eviction runs on every insert. The pass collects every entry's last-access mtime, sorts ascending, and removes entries until the directory total falls back under the configured cap *(default 100 MiB)*. The pass is best-effort and never blocks the insert, with permission failures and concurrent-eviction races logged to stderr as warnings.

## Configuration

Two knobs sit under `[tool.prose.cache]`:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the cache globally |
| `max-size-mib` | positive int | `100` | LRU eviction cap on the cache directory |

```toml
[tool.prose.cache]
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

## Corrupt-Entry Recovery

The cache silently degrades on read or write errors. A corrupt entry produces a cache miss, the runner falls through to a full pipeline run, and the corrupt entry is overwritten on the subsequent insert.

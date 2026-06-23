# CLI

The `prose` binary's subcommands each resolve a distinct workflow shape. `format` rewrites Python files in place, `check` reports violations without modifying anything, `server` speaks the language-server protocol to an editor, and `completions` emits a shell-completion script. `format` and `check` share the same path-handling, stdin, rule-filtering, and output-format surface, so a CI step that runs `prose check` and a developer that runs `prose format` see the same flag set with the same precedence.

## Synopsis

```bash
prose [--color WHEN] <SUBCOMMAND> [OPTIONS] [PATH...]
```

The global `--color` flag accepts `always`, `auto`, or `never` and applies to every subcommand's output. Subcommand-specific flags follow the subcommand name. Positional `PATH` arguments accept files or directories.

## `prose format`

Rewrites Python files to conform to the *Prose* style. Returns exit code 0 once the rewrites land, even when files changed.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--diff` | bool | off | Show a unified diff on stdout instead of writing changes |
| `--no-cache` | bool | off | Bypass the user-level [**cache**](/reference/cache) for this invocation |
| `--output-format` | `text` \| `json` \| `github` \| `sarif` | `text` | Diagnostic shape. `--diff` requires `text` |
| `--quiet` / `-q` | bool | off | Reduce the closing [**summary**](#run-summary) to a bare count line, dropping the anchor, color, and the `--diff` heading |
| `--stdin` | bool | off | Read source from stdin and write the rewritten source to stdout |
| `--stdin-filename` | path | unset | Treat stdin as this path, its extension selecting the source type. A `.ipynb` name reads stdin as a notebook |
| `--select` | comma-separated rule slugs | unset | Run only the listed rules, replacing the configured-enabled set |
| `--ignore` | comma-separated rule slugs | unset | Skip the listed rules, subtracting from whichever set would otherwise have run |
| `PATH...` | one or more paths, or `-` | required when not `--stdin` | Files or directories to format, or `-` to read source from stdin |

Exit codes: `0` clean / rewrites applied, `3` parse error, `4` config error *(see [**Exit Codes**](/reference/exit-codes))*.

```bash
prose format src/
prose format --diff src/
prose format --stdin < module.py
prose format - < module.py
prose format --select align-equals,align-colons src/
```

`--diff` emits a standard unified diff with three lines of context, suitable for piping into `patch`, `delta`, or any other diff-reading tool:

```diff
--- src/example.py
+++ src/example.py
@@ -1,5 +1,5 @@
 def configure():
-    timeout = 30
-    retries = 5
-    backoff_base = 1.5
+    timeout      = 30
+    retries      = 5
+    backoff_base = 1.5
```

## `prose check`

Reports violations without modifying source, returning the canonical [**Exit Codes**](/reference/exit-codes) matrix so CI gates pick up `1` *(format would change)* or `2` *(lint violation)* alongside the pass / parse / config codes. The flag table matches `prose format`'s above, omitting `--diff` because no rewrite is being staged for preview and adding `--validate` to opt into a rewrite-soundness pass:

| Flag | Type | Default | Description |
|---|---|---|---|
| `--no-cache` | bool | off | Bypass the user-level [**cache**](/reference/cache) for this invocation |
| `--output-format` | `text` \| `json` \| `github` \| `sarif` | `text` | Diagnostic shape. See [**Output Formats**](/reference/output-formats) for the per-format record layout |
| `--quiet` / `-q` | bool | off | Reduce the closing [**summary**](#run-summary) to a bare count line, dropping the anchor and color |
| `--stdin` | bool | off | Read source from stdin instead of the filesystem |
| `--stdin-filename` | path | unset | Treat stdin as this path, its extension selecting the source type. A `.ipynb` name reads stdin as a notebook |
| `--validate` | bool | off | Confirm each file's would-be rewrite re-parses, surfacing an unparseable rule output as a config error |
| `--select` | comma-separated rule slugs | unset | Run only the listed rules |
| `--ignore` | comma-separated rule slugs | unset | Skip the listed rules |
| `PATH...` | one or more paths, or `-` | required when not `--stdin` | Files or directories to check, or `-` to read source from stdin |

Exit codes: `0` clean, `1` format diagnostics pending, `2` lint diagnostics surfaced, `3` parse error, `4` config error.

```bash
prose check .
prose check --output-format github .
prose check --output-format sarif . > prose.sarif
prose check --stdin < module.py
prose check --validate .
prose check - < module.py
```

## Notebook inputs

`format` and `check` accept Jupyter notebooks (`.ipynb`) alongside `.py` files, both in path-mode discovery and through `--stdin-filename`, whose extension selects the source type. *Prose* parses the notebook, runs the pipeline once over the concatenated code-cell source, and re-emits the JSON with outputs, metadata, and cell structure preserved, rewriting only the code each cell holds. The sibling-reordering rules (`alphabetize`, `band-constants`, `group-imports`) stay out of a notebook run, because a cell's place in the execution order forbids moving code across a cell boundary. A non-Python notebook, an R or Julia kernel, is passed over the way an excluded path is skipped, and `format --diff` renders a unified diff per code cell under a cell header.

```bash
prose format notebook.ipynb
prose check analysis/
prose format --stdin --stdin-filename nb.ipynb < nb.ipynb
```

## `prose cache clean`

Removes every entry from the user-level cache and prints the freed bytes plus the cleared entry count. The [**Cache**](/reference/cache) reference covers the cache's location, key shape, and `[cache]` configuration.

```bash
prose cache clean
```

Returns exit code 0 on success, with the IO-error exit code applying on permission or filesystem failures.

## `prose cache compact`

Runs the LRU eviction pass against the cache, reducing it to the configured `[cache] max-size-mib` cap and reporting the bytes and entry count it removed. Useful after lowering the cap, since eviction otherwise runs only on insert.

```bash
prose cache compact
```

## `prose cache info`

Prints the cache directory's resolved path, total entry count, total byte size, and the oldest and newest entry mtimes as relative ages. Useful for verifying that `PROSE_CACHE_DIR` resolved where expected, or that the cache is being populated by recent runs.

```bash
prose cache info
```

## `prose completions`

Prints a shell-completion script to stdout for the shell named in the `<shell>` positional below.

| Positional | Values | Description |
|---|---|---|
| `<shell>` | `bash` \| `zsh` \| `fish` \| `elvish` \| `powershell` | Target shell |

```bash
prose completions zsh > "${fpath[1]}/_prose"
```

The [**Shell Completions**](/integrations/shell-completions) integration page covers the install path for each shell.

## `prose server`

Runs a language server over stdio, so an editor gets format-on-save and live rule diagnostics from the same binary it already installs. The server tracks each open buffer, runs the [**pipeline**](/reference/pipeline-order) over the editor's live text on a `textDocument/formatting` request, and republishes findings on every open and change. It resolves the workspace `[tool.prose]` [**configuration**](/reference/configuration) the way `prose check` does, so an editor session and a command-line run over the same tree agree on the active rule set.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--transport` | `stdio` | `stdio` | Transport the server speaks over. Only stdio is supported |

```bash
prose server
```

The [**Editor**](/integrations/editor) integration page covers pointing an editor's language-server client at the binary. Range and on-type formatting, code-action quick-fixes, and a bundled editor extension wait for a later pass, the first cut leaning on whole-document runs.

## Run Summary

Every interactive `check` or `format` run closes with a one-line summary on **stderr**, leaving stdout free for diagnostics, rewritten source, unified diffs, and the machine formats. Build an invocation to watch the line each outcome resolves to, across the run outcome, `--quiet`, and the stream's color state:

<RunSummaryExplorer />

A clean run anchors on 🪻, `check` violations on 🔖, and `format`'s applied or pending rewrites on 🗞️.

ANSI color draws on the project palette, with **Ube** on the anchor, **Celadon** on a clean count, and **Apricot** on a violation or change count. Each span renders as 24-bit color when the terminal advertises truecolor *(via `COLORTERM`)* and falls back to ANSI 8-color otherwise.

`--quiet` / `-q` reduces the line to its bare count *(`5 diagnostics in 2 files.`)*, dropping the anchor emoji and color, which is the shape a CI log wants. A non-TTY stderr keeps the anchored line but strips color, so a redirected run stays readable without escape noise. `--color never` strips color while leaving the anchor. Under `--output-format json`, `sarif`, or `github`, the machine output on stdout stays untouched by the summary.

`format --diff` heads each file's diff with a 🧵 `<path>` line on an interactive stdout. Off a TTY *(a pipe or redirect)* it keeps the plain `--- / +++` header instead, so the output stays a diff that `patch` and `delta` can read.

## Global Flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--color` | `always` \| `auto` \| `never` | `auto` | Colored output preference, applied to every subcommand |
| `--verbose` | bool | off | Print a one-line cache hit/miss summary to stderr at the end of each `check` or `format` run |

`--color auto` honors the [**`NO_COLOR`**](https://no-color.org/) environment variable and the terminal's TTY status. `--color always` forces ANSI sequences even when stdout is not a TTY *(useful for piping to `less -R`)*. `--color never` strips ANSI sequences unconditionally.

`--verbose` writes one line of cache telemetry to stderr: `cache: N hits, M misses, T files`, or `cache: bypassed` when the cache is disabled. The [**Cache**](/reference/cache#hit-miss-telemetry) page covers the shape.

## Mutual Exclusion

`--stdin` and `PATH...` are mutually exclusive on both `format` and `check`. A run that passes both fails at clap-parse time with exit code 4. The `-` positional alias for stdin obeys the same restrictions, in that `prose check - --stdin` and `prose format - a.py` both fail at parse time. `--diff` is mutually exclusive with any non-text `--output-format`, since the diff is itself the text-format presentation.

## Precedence

`--select` and `--ignore` compose against the configured-enabled set as **select minus ignore**. With no `--select`, every configured-enabled rule runs except those listed in `--ignore`. With a `--select` set, only the listed rules run, then `--ignore` subtracts. A `--select` value that names a configured-disabled rule re-enables it for that one invocation, which is useful when debugging the effect of a rule that the project has globally turned off:

```bash
prose check --select align-equals src/
```

This runs `align-equals` against `src/` even when `[rules]` has `align-equals = false`. The [**Configuration**](/reference/configuration) reference covers the per-rule toggle.

CLI flags are per-invocation only, so none of the flags above *(including `--output-format`, `--color`, `--diff`, `--select`, `--ignore`)* can be set in `[tool.prose]`. The configuration file carries semantic knobs *(line lengths, per-rule toggles, rule-specific inputs)*, and the CLI carries invocation knobs *(input source, output shape, color)*.

The [**Quick Start**](/usage/quick-start) chapter walks through the most common invocations, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the deterministic order rules fire in.

# CLI

Each `prose` subcommand does one job, in that `format` rewrites Python files in place, `check` reports what would change without writing anything, `server` runs a language server for an editor, and `completions` prints a shell-completion script. `format` and `check` take the same path arguments, the same stdin flags, the same rule-filtering flags, and the same output formats, so a CI step running `prose check` and a developer running `prose format` share one flag set and one set of rules for how the flags combine.

## Synopsis

```bash
prose [--color WHEN] <SUBCOMMAND> [OPTIONS] [PATH...]
```

The global `--color` flag takes `always`, `auto`, or `never` and applies to every subcommand. A subcommand's own flags follow its name, and each `PATH` argument names a file or a directory.

## `prose format`

Rewrites Python files to the *Prose* style and exits 0 once the rewrites are written, whether or not any file changed.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--diff` | bool | off | Print a unified diff to stdout instead of writing the files |
| `--no-cache` | bool | off | Bypass the per-user [**cache**](/reference/cache) for this run |
| `--output-format` | `text` \| `json` \| `github` \| `sarif` | `text` | The diagnostic format. `--diff` needs `text` |
| `--quiet` / `-q` | bool | off | Reduce the closing [**summary**](#run-summary) to a bare count, dropping the anchor, the color, and the `--diff` heading. An [**unstable-output notice**](#unstable-output) still prints |
| `--stdin` | bool | off | Read source from stdin and write the rewritten source to stdout |
| `--stdin-filename` | path | unset | Treat stdin as this path, whose extension selects the source type. A `.ipynb` name reads stdin as a notebook |
| `--select` | comma-separated rule slugs | unset | Run only the listed rules, replacing the configured set |
| `--ignore` | comma-separated rule slugs | unset | Skip the listed rules, removing them from whichever set would otherwise run |
| `PATH...` | one or more paths, or `-` | required without `--stdin` | Files or directories to format, or `-` to read source from stdin |

Exit codes: `0` clean or rewrites written, `1` a `--diff` run found a pending rewrite, `2` lint findings reported, `3` parse error, `4` config error *(see [**Exit Codes**](/reference/exit-codes))*. A rewrite the [**settle check**](#unstable-output) reports as unstable is still written and still sets the exit code as any other rewrite would, with its notice going to stderr rather than changing the code.

```bash
prose format src/
prose format --diff src/
prose format --stdin < module.py
prose format - < module.py
prose format --select align-equals,align-colons src/
```

`--diff` prints a standard unified diff with three lines of context, which `patch`, `delta`, and any other diff reader accept:

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

Reports what would change without writing anything, so a CI gate reads `1` *(a rewrite is pending)* or `2` *(a lint finding)* alongside the pass, parse-error, and config-error codes in the [**Exit Codes**](/reference/exit-codes) reference. The flags match `prose format`'s above, minus `--diff`, since nothing is written for a diff to preview, and plus `--validate`, which checks each file's would-be rewrite for soundness:

| Flag | Type | Default | Description |
|---|---|---|---|
| `--no-cache` | bool | off | Bypass the per-user [**cache**](/reference/cache) for this run |
| `--output-format` | `text` \| `json` \| `github` \| `sarif` | `text` | The diagnostic format. See [**Output Formats**](/reference/output-formats) for each format's record layout |
| `--quiet` / `-q` | bool | off | Reduce the closing [**summary**](#run-summary) to a bare count, dropping the anchor and the color. An [**unstable-output notice**](#unstable-output) still prints |
| `--stdin` | bool | off | Read source from stdin instead of the filesystem |
| `--stdin-filename` | path | unset | Treat stdin as this path, whose extension selects the source type. A `.ipynb` name reads stdin as a notebook |
| `--validate` | bool | off | Confirm each file's would-be rewrite parses and settles under every enabled rule *(whereas `format` re-applies only the rules that edited)*, and report a rule output that fails to parse or an [**unstable one**](#unstable-output) as a config error |
| `--select` | comma-separated rule slugs | unset | Run only the listed rules |
| `--ignore` | comma-separated rule slugs | unset | Skip the listed rules |
| `PATH...` | one or more paths, or `-` | required without `--stdin` | Files or directories to check, or `-` to read source from stdin |

Exit codes: `0` clean, `1` a rewrite is pending, `2` lint findings reported, `3` parse error, `4` config error.

```bash
prose check .
prose check --output-format github .
prose check --output-format sarif . > prose.sarif
prose check --stdin < module.py
prose check --validate .
prose check - < module.py
```

## Notebook Inputs

`format` and `check` accept Jupyter notebooks (`.ipynb`) alongside `.py` files, both when walking a directory and through `--stdin-filename`, whose extension selects the source type. *Prose* parses the notebook, runs the pipeline once over the code cells joined together, and writes the JSON back with outputs, metadata, and cell structure unchanged, rewriting only the code inside each cell. A notebook whose kernel is not Python, such as R or Julia, is skipped the way an excluded path is.

The rules that reorder siblings ([[alphabetize-siblings]], [[band-constants]], [[group-imports]]) sort and band within each cell and never move a statement across a cell boundary, because a cell's position in the execution order fixes where its code may run. Within a cell, [[band-constants]] goes further, banding only a constant whose value runs no code *(a literal, a name, an attribute or subscript read, a display or operator expression over these, or a `lambda`)* and leaving in place any value containing a call, a comprehension, or an `await`, because evaluating such a value runs code and moving it past the cell's other statements would change what the cell computes.

`format --diff` prints one unified diff per code cell under a `cell N` header, and `check` reports each finding against its own cell, with the text format printing the same `cell N` header and the JSON record carrying the cell number beside a position relative to that cell. Every output numbers a cell by its absolute position in the notebook, counting Markdown cells too, so one code cell has the same number in a diff header, a text report, and a JSON record.

```bash
prose format notebook.ipynb
prose check analysis/
prose format --stdin --stdin-filename nb.ipynb < nb.ipynb
```

## `prose cache clean`

Deletes every entry in the per-user cache and prints the bytes freed and the number of entries deleted. The [**Cache**](/reference/cache) reference covers the cache's location, its key, and the `[cache]` configuration.

```bash
prose cache clean
```

It exits `0` on success and `4` on a permission or filesystem failure, with the error printed to stderr.

## `prose cache compact`

Runs the eviction pass immediately, reducing the cache to the configured `[cache] max-size-mib` and `max-entries` caps and printing the bytes and entries it removed. It is useful after lowering either cap, since eviction otherwise runs only at the end of a run over paths.

```bash
prose cache compact
```

## `prose cache info`

Prints the cache directory's resolved path, its entry count, its total size in bytes, and the mtimes of its oldest and newest entries as relative ages. It shows whether `PROSE_CACHE_DIR` resolved to the expected path and whether recent runs are writing entries.

```bash
prose cache info
```

## `prose completions`

Prints a completion script for the shell named by the `<shell>` argument:

| Positional | Values | Description |
|---|---|---|
| `<shell>` | `bash` \| `zsh` \| `fish` \| `elvish` \| `powershell` | The shell to generate for |

```bash
prose completions zsh > "${fpath[1]}/_prose"
```

The [**Shell Completions**](/integrations/shell-completions) integration page covers where each shell reads the script from.

## `prose rules`

Lists every registered rule in pipeline order, one row per rule with its one-based position, its slug, and its imperative. The JSON form prints the same list as an array of `{after, imperative, position, slug}` records for a script to read.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--output-format` | `table` \| `json` | `table` | The listing format |

```bash
prose rules
prose rules --output-format json
```

The [**Pipeline Order**](/reference/pipeline-order) reference covers how the registry orders the rules the listing shows.

## `prose schema`

Prints the [JSON Schema](https://json-schema.org) for the `[tool.prose]` configuration, with each key's type, default, allowed values, and numeric range, and each `[rules]` entry in the bare-bool-or-table shape the loader accepts. The output is pretty-printed, so redirecting it to a file gives an editor or a validator a schema it can read directly.

```bash
prose schema
prose schema > prose.schema.json
```

The [**Configuration**](/reference/configuration) reference walks through the keys the schema describes.

## `prose server`

Runs a language server over stdio, so an editor gets format-on-save and live diagnostics from the binary it already has. The server tracks each open buffer, runs the [**pipeline**](/reference/pipeline-order) over the editor's live text on a `textDocument/formatting` request, and publishes findings again on every open and change. It resolves the workspace `[tool.prose]` [**configuration**](/reference/configuration) the way `prose check` does, so an editor session and a command-line run over the same tree use the same rule set.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--transport` | `stdio` | `stdio` | The transport the server speaks over. Only stdio is supported |

```bash
prose server
```

The [**Editor**](/integrations/editor) integration page covers pointing an editor's language-server client at the binary. The server formats whole documents only, with range formatting, on-type formatting, code-action quick fixes, and a bundled editor extension not yet implemented.

## Unstable Output

Running *Prose* twice leaves the second run nothing to do, for whichever subset of rules a project enables and not only for the default set. A `format` run that rewrites a file therefore re-applies its enabled rules to the output it just wrote, and where any rule still changes it, the run prints a notice on **stderr** naming the defect as *Prose*'s own:

```console
$ prose format src/module.py
🐞 prose rewrote src/module.py to output a second run would change.

The defect lies in prose rather than in the file, so reproduce it now and confirm it
gone after an upgrade with:

    prose format --select align-equals src/module.py

Filing is one click, the form already carrying the version, the reproducing slugs, and
the resolved configuration, with anything too long for a link left to paste:

    https://github.com/Jybbs/prose/issues/new?template=unstable-output.yml&…

--- src/module.py (first pass)
+++ src/module.py (second pass)
@@ -1,3 +1,3 @@
-alpha     = 1
-beta      = 2
-long_name = 3
+alpha      = 1
+beta       = 2
+long_name  = 3

🗞️ Reformatted 1 file.
🐞 1 file would change on a second run.
```

The `--select` list names the smallest subset that still reproduces the defect rather than every rule that ran, narrowed to one rule where one suffices and to a pair where only two rules together disagree. The search runs under a fixed budget and falls back to naming every rule that ran where no single rule or pair reproduces within it, and a notebook's report names the whole selection, since its cells do not go through the narrowing step. The same `--select` command confirms the fix after an upgrade, so nobody has to wait for a release note. Each run rewrites the file again, so capturing the two passes for a report goes through the `--stdin` form the [**issue form**](https://github.com/Jybbs/prose/issues/new?template=unstable-output.yml) itself shows.

The rewrite is written and the exit code reflects that rewrite alone, because the defect belongs to the formatter rather than to the source. A project that would rather fail CI on the finding uses [`prose check --validate`](#prose-check), which prints the same notice and exits `4` as its other validation failures do.

When the [**cache**](/reference/cache) is live, a `format` run that rewrites a file skips the settle check and records the bytes it wrote in the cache's ledger as its own output, so the run that next reads those bytes and finds them changing again is the one that reports the defect. That run reports it at the moment the symptom first appears and with the smallest reproducing source at hand. A `--diff` preview, a `check --validate`, and any `--no-cache` run keep the same-run check, so a workflow that needs the notice before anything is written has one.

A notice survives the [**cache**](/reference/cache), because the entry a run stores carries the report beside the diagnostics and the rewrite, so a hit prints the block again rather than losing it with the skipped pipeline run. Reaching for `--no-cache` to trust the notice is unnecessary.

A run over a tree groups its notices so the output stays readable. Files reproducing under one subset collapse into one block naming how many there are and giving the command for the first of them, the second-pass diff prints only where a block covers one file, and the closing summary gains a 🐞 line counting the files whose output a second run would change. `--quiet` trims routine output rather than a defect notice, so the block still prints, with the anchor and color stripped the way every other line's are.

`report-unstable-output = false` in `[tool.prose]` turns the notice off for every `format` run and for the editor message, and the [**Configuration**](/reference/configuration#top-level-keys) reference covers the key alongside every other. The key governs the notice alone, so a `check --validate` run still performs the settle check the flag requests.

## Run Summary

Every interactive `check` or `format` run ends with a one-line summary on **stderr**, leaving stdout for diagnostics, rewritten source, unified diffs, and the machine-readable formats. Build a command below to see the line each outcome produces, across the run outcome, `--quiet`, and whether the stream is colored:

<RunSummaryExplorer />

Each outcome opens on its own anchor:

| Anchor | Outcome |
|---|---|
| 🪻 | A clean run |
| 🔖 | `check` findings, or a `format` run's unfixed lints |
| 🗞️ | `format`'s written or pending rewrites |
| 🐞 | A rewrite the settle check reported |

ANSI color uses the project palette, with **Ube** on the anchor, **Celadon** on a clean count, and **Apricot** on a finding or change count. Each span prints in 24-bit color when the terminal advertises truecolor *(through `COLORTERM`)* and in ANSI 8-color otherwise.

`--quiet` / `-q` reduces the line to its bare count *(`5 diagnostics in 2 files.`)*, dropping the anchor emoji and the color, which is the form a CI log captures cleanly. A non-TTY stderr keeps the anchor but drops the color, so a redirected run stays readable without escape sequences. `--color never` drops the color and keeps the anchor. Under `--output-format json`, `sarif`, or `github`, the machine-readable output on stdout is unaffected by the summary.

`format --diff` opens each file's diff with a 🧵 `<path>` line on an interactive stdout. On a pipe or a redirect it prints the plain `--- / +++` header instead, so the output stays a diff `patch` and `delta` can read.

## Global Flags

| Flag | Type | Default | Description |
|---|---|---|---|
| `--color` | `always` \| `auto` \| `never` | `auto` | Whether to color output, applied to every subcommand |
| `--verbose` | bool | off | Print one line of cache hit and miss counts to stderr at the end of each `check` or `format` run |

`--color auto` reads the [**`NO_COLOR`**](https://github.com/jcs/no_color) environment variable and whether stdout is a TTY. `--color always` writes ANSI sequences even when stdout is not a TTY *(useful when piping to `less -R`)*. `--color never` writes none.

`--verbose` writes one line of cache counts to stderr, `cache: N hits, M misses, T files`, or `cache: bypassed` when the cache is off. The [**Cache**](/reference/cache#hit-miss-telemetry) page covers the line.

## Mutual Exclusion

`--stdin` and `PATH...` cannot be combined on `format` or `check`. A run that passes both fails while parsing its arguments with exit code 4. The `-` argument for stdin follows the same rule, so `prose check - --stdin` and `prose format - a.py` both fail the same way. `--diff` cannot be combined with any `--output-format` other than `text`, since the diff is the text format's own presentation.

## Precedence

`--select` and `--ignore` combine with the configured rule set as **select minus ignore**. Without `--select`, every rule the configuration enables runs except those listed in `--ignore`. With `--select`, only the listed rules run, and `--ignore` then removes any of them it names. A `--select` naming a rule the configuration turned off runs that rule for the one invocation, which is useful for seeing what a rule the project has turned off would do:

```bash
prose check --select align-equals src/
```

This runs `align-equals` on `src/` even when `[rules]` has `align-equals = false`. The [**Configuration**](/reference/configuration) reference covers the per-rule toggle.

Every flag above is per invocation, so none of them *(`--output-format`, `--color`, `--diff`, `--select`, and `--ignore` included)* can be set in `[tool.prose]`. The configuration file carries the semantic settings *(line lengths, per-rule toggles, per-rule inputs)*, and the command line carries the invocation settings *(input source, output format, color)*.

The [**Quick Start**](/usage/quick-start) chapter walks through the most common commands, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the order rules run in.

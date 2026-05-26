# CLI

The `prose` binary's subcommands each resolve a distinct workflow shape. `format` rewrites Python files in place, `check` reports violations without modifying anything, and `completions` emits a shell-completion script. `format` and `check` share the same path-handling, stdin, rule-filtering, and output-format surface, so a CI step that runs `prose check` and a developer that runs `prose format` see the same flag set with the same precedence.

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
| `--stdin` | bool | off | Read source from stdin and write the rewritten source to stdout |
| `--select` | comma-separated rule slugs | unset | Run only the listed rules, replacing the configured-enabled set |
| `--ignore` | comma-separated rule slugs | unset | Skip the listed rules, subtracting from whichever set would otherwise have run |
| `PATH...` | one or more paths | required when not `--stdin` | Files or directories to format |

Exit codes: `0` clean / rewrites applied, `3` parse error, `4` config error *(see [**Exit Codes**](/reference/exit-codes))*.

```bash
prose format src/
prose format --diff src/
prose format --stdin < module.py
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

Reports violations without modifying source. Returns the canonical [**Exit Codes**](/reference/exit-codes) matrix so CI gates pick up `1` *(format would change)* or `2` *(lint violation)* alongside the pass / parse / config codes. The flag table matches `prose format`'s above, omitting `--diff` because no rewrite is being staged for preview:

| Flag | Type | Default | Description |
|---|---|---|---|
| `--no-cache` | bool | off | Bypass the user-level [**cache**](/reference/cache) for this invocation |
| `--output-format` | `text` \| `json` \| `github` \| `sarif` | `text` | Diagnostic shape. See [**Output Formats**](/reference/output-formats) for the per-format record layout |
| `--stdin` | bool | off | Read source from stdin instead of the filesystem |
| `--select` | comma-separated rule slugs | unset | Run only the listed rules |
| `--ignore` | comma-separated rule slugs | unset | Skip the listed rules |
| `PATH...` | one or more paths | required when not `--stdin` | Files or directories to check |

Exit codes: `0` clean, `1` format diagnostics pending, `2` lint diagnostics surfaced, `3` parse error, `4` config error.

```bash
prose check .
prose check --output-format github .
prose check --output-format sarif . > prose.sarif
prose check --stdin < module.py
```

## `prose cache clean`

Removes every entry from the user-level cache and prints the freed bytes plus the cleared entry count. The [**Cache**](/reference/cache) reference covers the cache's location, key shape, and `[tool.prose.cache]` configuration.

```bash
prose cache clean
```

Returns exit code 0 on success. The IO-error exit code applies on permission or filesystem failures.

## `prose completions`

Prints a shell-completion script to stdout.

| Positional | Values | Description |
|---|---|---|
| `<shell>` | `bash` \| `zsh` \| `fish` \| `elvish` \| `powershell` | Target shell |

```bash
prose completions zsh > "${fpath[1]}/_prose"
```

The [**Shell Completions**](/integrations/shell-completions) integration page covers the install path for each shell.

## Global Flag

| Flag | Type | Default | Description |
|---|---|---|---|
| `--color` | `always` \| `auto` \| `never` | `auto` | Colored output preference, applied to every subcommand |

`--color auto` honors the [**`NO_COLOR`**](https://no-color.org/) environment variable and the terminal's TTY status. `--color always` forces ANSI sequences even when stdout is not a TTY *(useful for piping to `less -R`)*. `--color never` strips ANSI sequences unconditionally.

## Mutual Exclusion

`--stdin` and `PATH...` are mutually exclusive on both `format` and `check`. A run that passes both fails at clap-parse time with exit code 4. `--diff` is mutually exclusive with any non-text `--output-format`, since the diff is itself the text-format presentation.

## Precedence

`--select` and `--ignore` compose against the configured-enabled set as **select minus ignore**. With no `--select`, every configured-enabled rule runs except those listed in `--ignore`. With a `--select` set, only the listed rules run, then `--ignore` subtracts. A `--select` value that names a configured-disabled rule re-enables it for that one invocation, which is useful when debugging the effect of a rule that the project has globally turned off:

```bash
prose check --select align-equals src/
```

This runs `align-equals` against `src/` even when `[tool.prose.rules.align-equals]` has `enabled = false`. The [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` knob.

CLI flags are per-invocation only. None of the flags above *(including `--output-format`, `--color`, `--diff`, `--select`, `--ignore`)* can be set in `[tool.prose]`. The configuration file carries semantic knobs *(line lengths, per-rule toggles, rule-specific inputs)*, and the CLI carries invocation knobs *(input source, output shape, color)*.

The [**Quick Start**](/usage/quick-start) chapter walks through the most common invocations, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the deterministic order rules fire in.

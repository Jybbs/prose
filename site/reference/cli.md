# CLI

The `prose` binary exposes **three** subcommands, wherein each one resolves a distinct workflow shape. `format` rewrites Python files in place, `check` reports violations without modifying anything, and `completions` emits a shell-completion script. `format` and `check` share the same path-handling, stdin, rule-filtering, and output-format surface, so a CI step that runs `prose check` and a developer that runs `prose format` see the same flag set with the same precedence.

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

## `prose check`

Reports violations without modifying source. Returns the canonical [**Exit Codes**](/reference/exit-codes) matrix so CI gates pick up `1` *(format would change)* or `2` *(lint violation)* alongside the pass / parse / config codes.

| Flag | Type | Default | Description |
|---|---|---|---|
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

`--select` and `--ignore` compose against the configured-enabled set as **select minus ignore**. With no `--select`, every configured-enabled rule runs except those listed in `--ignore`. With a `--select` set, only the listed rules run, then `--ignore` subtracts. A `--select` value that names a configured-disabled rule re-enables it for that one invocation. The [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` knob.

The [**Quick Start**](/guide/quick-start) chapter walks through the most common invocations, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the deterministic order rules fire in.

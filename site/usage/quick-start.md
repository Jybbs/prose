# Quick Start

Three subcommands cover every shape of run *Prose* supports. `format` rewrites files in place, `check` reports violations without modifying anything, and `completions` emits a shell-completion script. The same exit-code matrix gates both `format` and `check`, meaning a CI step and a local pre-commit hook compile against the same outcomes.

## Run on a Project

The canonical invocation is `prose format path/to/project`, which walks the directory tree, runs every enabled rule, and writes the settled layout back to disk:

```bash
prose format path/to/project
```

Three variants cover the surrounding cases. `prose check` is the CI shape, doing the same walk against the same rules but reporting diagnostics to stdout and gating on the exit code without touching files. `prose format --diff` is the previewing shape, emitting a unified diff to stdout in lieu of writing changes. `prose check --stdin` *(also `prose format --stdin`)* takes one file's contents on stdin and routes diagnostics or rewrites to stdout, which is the shape an editor wires into a save hook.

```bash
prose check path/to/project
prose format --diff path/to/project
prose check --stdin < file.py
```

## Which Files Get Walked

When a path is a directory, *Prose* walks it through an ignore-aware walker that honors `.gitignore`, `.ignore`, and the user's global ignore file by default. The vendored dependencies, build artifacts, and any other paths a `.gitignore` covers stay out of the run automatically, so `prose format .` against the project root matches `git ls-files` minus the binary excludes.

For a path that should be skipped without showing up in `.gitignore` *(a generated directory the project commits, a migrations folder, a third-party snapshot)*, drop the path into `.ignore` at the project root or any directory above the path being walked. *Prose* doesn't ship a separate `.proseignore` file or an `exclude` config key, because the `.ignore` convention from `ripgrep` and `fd` already covers the case cleanly and the walker reads it without further configuration.

Hidden files and directories *(anything starting with `.`)* are walked too, with the same gitignore semantics applied. The walk is bounded by the path arguments themselves, so `prose format src/` confines the walk to `src/` even when the broader project has a different layout. The [[walker]] primitive page covers the internal machinery and the multi-root pattern the CLI consumes.

## Subset the Active Rules

`--select` and `--ignore` restrict the run to a subset of the configured rules. `--select` replaces the configured-enabled set entirely, so `--select align-equals` runs only `align-equals` regardless of what `[tool.prose]` toggled on or off. `--ignore` subtracts from whichever set would otherwise have run, so `--ignore strip-trailing-commas` drops one rule while leaving everything else in the configured set active. When both flags appear, *Prose* composes them as `select - ignore`, applying the select first and then removing the ignored rules from the result.

```bash
prose check --select align-equals path/
prose check --ignore strip-trailing-commas path/
prose check --select align-equals,align-colons path/
```

The [**Rules**](/rules/) page enumerates every rule the binary ships, with one page per rule walking the canonical case and the surrounding behavior. The [**CLI Reference**](/reference/cli) covers every flag with its precedence rules and exit codes.

## Parallelism

Path-mode runs parallelize across files via [**`rayon`**](https://docs.rs/rayon/), with one rule pipeline per worker thread, leaving large repos to settle in wall-clock time proportional to the slowest file rather than the file count. Setting `RAYON_NUM_THREADS=1` forces single-threaded execution, which is the shape to reach for when debugging a rule whose diagnostic output reads as confusing under parallel emission. Stdin mode is single-threaded by construction, because the input is one file and there's nothing to parallelize across.

## Where to Go Next

The [**Ruff**](/integrations/ruff) integration page covers pairing *Prose* with Ruff for projects that run both. The [**Suppression**](/usage/suppression) chapter covers per-line and block-level opt-outs. The [**Exit Codes**](/reference/exit-codes) reference is the source-of-truth for CI gating.

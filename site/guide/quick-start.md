# Quick Start

Three subcommands cover every shape of run *Prose* supports. `format` rewrites files in place, `check` reports violations without modifying anything, and `completions` emits a shell-completion script. The same exit-code matrix gates both `format` and `check`, meaning a CI step and a local pre-commit hook compile against the same outcomes. Path-mode runs parallelize across files via [**`rayon`**](https://docs.rs/rayon/), with one rule pipeline per worker thread, leaving large repos to settle in wall-clock time proportional to the slowest file rather than the file count.

## Run on a Project

Rewrite a tree of Python files in place:

```bash
prose format path/to/project
```

Check without rewriting *(the CI shape)*:

```bash
prose check path/to/project
```

Show the rewrite as a unified diff without touching files *(useful for previewing)*:

```bash
prose format --diff path/to/project
```

Read from stdin, write to stdout *(useful for editor integration)*:

```bash
prose check --stdin < file.py
```

## Which Files Get Walked

When a path is a directory, *Prose* walks it through an ignore-aware walker that honors `.gitignore`, `.ignore`, and the user's global ignore file by default. The vendored dependencies, build artifacts, and any other paths a `.gitignore` covers stay out of the run automatically, so `prose format .` against the project root matches `git ls-files` minus the binary excludes.

Hidden files and directories *(anything starting with `.`)* are walked too, with the same gitignore semantics applied. The walk is bounded by the path arguments themselves, so `prose format src/` confines the walk to `src/` even when the broader project has a different layout.

The [[walker]] primitive page covers the internal machinery and the multi-root pattern the CLI consumes.

## Subset the Active Rules

`--select` and `--ignore` restrict the run to a subset of the configured rules. Use `--select` to run only one or two rules *(useful for incremental adoption)*, and `--ignore` to disable specific rules for one invocation *(useful for debugging an unexpected diff)*:

```bash
prose check --select align-equals path/
prose check --ignore strip-trailing-commas path/
prose check --select align-equals,align-colons path/
```

The [**Rules Overview**](/rules/) page enumerates every rule the binary ships, with one page per rule walking the canonical case and the surrounding behavior. The [**CLI Reference**](/reference/cli) covers every flag with its precedence rules and exit codes.

## Where to Go Next

The [**Two-Stage Pipeline**](/guide/two-stage-pipeline) chapter covers pairing *Prose* with [**Ruff**](https://docs.astral.sh/ruff/) for the canonical layout-after-tokens flow. The [**Suppression**](/guide/suppression) chapter covers per-line and block-level opt-outs. The [**Exit Codes**](/reference/exit-codes) reference is the source-of-truth for CI gating.

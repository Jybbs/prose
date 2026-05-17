# Installation

*Prose* ships as a single binary that runs on Linux, macOS, and Windows. The recommended path is through [**`uv`**](https://docs.astral.sh/uv/), which fetches the platform wheel and exposes the `prose` executable on the user's `PATH` without an explicit venv.

## Install

```bash
uv tool install prose-formatter
```

Confirm the install with:

```bash
prose --version
```

`pip install prose-formatter` and `pipx install prose-formatter` work the same way for users who prefer those package managers. The PyPI distribution is the same wheel in every case, so the install path is whatever fits the project's existing tooling.

## Quick Start

Rewrite a tree of Python files in place:

```bash
prose format path/to/project
```

Check without rewriting (*the CI shape*):

```bash
prose check path/to/project
```

Show the rewrite as a unified diff without touching files (*useful for previewing*):

```bash
prose format --diff path/to/project
```

Read from stdin, write to stdout (*useful for editor integration*):

```bash
prose check --stdin < file.py
```

## Two-Stage Pipeline

*Prose* runs as the layout pass in a two-stage pipeline. Pair it with a token-level formatter that owns line wrapping, quote normalization, and indentation. [**Ruff**](https://docs.astral.sh/ruff/) is the canonical first pass, since `ruff format` covers the token surface that *Prose* doesn't touch.

```bash
ruff format && prose format
```

::: warning Order matters
Run *Prose* first and an upstream re-wrap will undo your per-line layout decisions, forcing a third pass, because the alignment math depends on already-settled line breaks.
:::

See the [**CI Integration**](/guide/ci-integration) chapter for shaping this into a GitHub Actions workflow, the [**Editor Integration**](/guide/editor-integration) chapter for run-on-save wiring, and the [**Configuration**](/guide/configuration) chapter for per-rule knobs.

## Which Files Get Walked

When a path is a directory, *Prose* walks it through an ignore-aware walker that honors `.gitignore`, `.ignore`, and the user's global ignore file by default. The vendored dependencies, build artifacts, and any other paths a `.gitignore` covers stay out of the run automatically, so `prose format .` against the project root matches `git ls-files` minus the binary excludes.

Hidden files and directories (*anything starting with `.`*) are walked too, with the same gitignore semantics applied. The walk is bounded by the path arguments themselves, so `prose format src/` confines the walk to `src/` even when the broader project has a different layout.

## Subset the Active Rules

`--select` and `--ignore` restrict the run to a subset of the configured rules. Use `--select` to run only one or two rules (*useful for incremental adoption*), and `--ignore` to disable specific rules for one invocation (*useful for debugging an unexpected diff*):

```bash
prose check --select align-equals path/
prose check --ignore strip-trailing-commas path/
prose check --select align-equals,align-colons path/
```

The [**Rules Overview**](/rules/) page enumerates every rule the binary ships, with one page per rule walking the canonical case and the surrounding behavior.

## Exit Codes

The binary resolves every run into one of five exit codes that CI gates compile against. When two outcomes apply to the same run, the higher number wins. Click a row for the specific rules and CLI invocations that produce each code.

<ExitCodeMatrix />

The CLI's `--help` prints the same matrix beneath the option list. For the gate semantics in CI, see the [**CI Integration**](/guide/ci-integration#exit-codes) chapter.

## Shell Completions

```bash
prose completions zsh > "${fpath[1]}/_prose"
prose completions bash > /etc/bash_completion.d/prose
prose completions fish > ~/.config/fish/completions/prose.fish
```

Both `elvish` and `powershell` are supported targets for the `completions` subcommand.

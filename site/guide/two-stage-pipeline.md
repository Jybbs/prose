# Two-Stage Pipeline

*Prose* runs as the **layout pass** in a two-stage pipeline. The token-level pass owns line wrapping, quote normalization, indentation, and blank-line discipline. The layout pass *(this one)* owns alignment, alphabetization, the singleton rule, one-entry-per-line collections, and trailing-comma stripping. The two passes don't overlap, so a project that runs both gets a settled token surface from the first pass and a settled layout surface from the second.

## The Canonical Sequence

<Tool slug="ruff" /> is the canonical first pass, because `ruff format` covers the token surface that *Prose* doesn't touch. Run Ruff first, then *Prose*:

```bash
ruff format && prose format
```

::: warning Order Matters
Running *Prose* first is incorrect, because *Prose*'s alignment math depends on already-settled line breaks and an upstream re-wrap will undo per-line layout decisions, forcing a third pass.
:::

The [**Ruff integration page**](/integrations/ruff) carries the full Ruff pairing reference, with the conflicting `pycodestyle` codes that need `extend-ignore` and a copy-ready `ruff.toml` config block. Other token-level formatters *(Black, autopep8)* pair with *Prose* through the same shape, with the same "run the token pass first, *Prose* second" discipline.

## In CI

The same two-stage pipeline compiles into CI, with each tool running in `--check` / `check` mode so the gate fails on any pending rewrite without writing to the runner's filesystem:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff format --check .
- run: prose check .
```

The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow-command annotation form, the SARIF upload pattern, and the canonical PR check shape. The [**Pre-Commit**](/integrations/pre-commit) integration page covers the hook configuration for both the local two-stage pattern and the upstream Ruff hook.

## In an Editor

For run-on-save, the order matters the same way the CLI does. The [**Editor**](/integrations/editor) integration page covers VSCode, Neovim, JetBrains, Sublime Text, Emacs, and Helix, each wiring the same `ruff format <file> && prose format <file>` shape into the save hook. Editors consuming structured diagnostics through `prose check --output-format json --stdin` see the same `code` / `message` / `location` shape Ruff publishes, so a Ruff-shaped diagnostic surface picks up *Prose* without adapter work.

# Ruff

<Tool slug="ruff" /> is the canonical first pass in *Prose*'s [**two-stage pipeline**](/guide/two-stage-pipeline). This page covers the Ruff-specific configuration that keeps the two tools from fighting each other.

## The Canonical Sequence

```bash
ruff format && prose format
```

::: warning Order Matters
Running *Prose* first is incorrect. *Prose*'s alignment math depends on already-settled line breaks, in that an upstream re-wrap will undo per-line layout decisions and force a third pass.
:::

A second run of `ruff format` against *Prose*'s output is a no-op, because the alignment padding *Prose* introduces lives within the lines Ruff already settled, leaving Ruff with nothing to re-wrap. The two-tool sequence is idempotent at the pair level, meaning a developer can re-run the canonical command after a manual edit without expecting either tool to thrash.

## Ruff Configuration

A handful of Ruff's `pycodestyle` codes flag whitespace patterns that *Prose*'s alignment rules deliberately introduce, so a clean two-tool run needs `extend-ignore` to silence them on the Ruff side. Copy this block into the project's `ruff.toml` *(or under `[tool.ruff]` in `pyproject.toml`)*:

```toml
[lint]
extend-ignore = [
  "COM812",  # trailing commas
  "E203",    # space before `:`
  "E221",    # space before `=`
  "E272",    # space before `import` / `as`
  "E501",    # line length
]

[format]
skip-magic-trailing-comma = true
```

The conflict table:

| Code | Conflict | Reason |
|---|---|---|
| `COM812` | Lint re-adds trailing commas | [[strip-trailing-commas]] removes them in multi-line collections and signatures |
| `E203` | Lint flags whitespace before `:` | [[align-colons]] produces it in dict literals, dataclass fields, function signatures, and docstring `Args:` blocks |
| `E221` | Lint flags multiple spaces before `=` | [[align-equals]] produces it across consecutive assignments at the same indentation |
| `E272` | Lint flags multiple spaces before `import` / `as` | [[align-imports]] produces it across `from ... import ...` and `import ... as ...` groups |
| `E501` | Lint flags lines past `line-length` | A long member in an alignment group pads shorter lines rightward, occasionally past the configured limit |
| `skip-magic-trailing-comma` | Formatter re-expands collections by trailing-comma presence | `prose format` controls collection layout independently of comma signaling, via [[collection-layout]] |

## In CI

The pipeline compiles into CI as two sequential check steps, with the exit codes gating the workflow. The <Tool slug="github" /> integration page covers the workflow shape end-to-end:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff format --check .
- run: prose check .
```

A non-zero exit from either step fails the gate. The [**GitHub Actions**](/integrations/github-actions) integration page covers the annotation forms and SARIF upload.

## In an Editor

For run-on-save in editors, chain the commands in the save hook so Ruff settles tokens before *Prose* lays out lines:

```bash
ruff format ${file} && prose format ${file}
```

The [**Editor**](/integrations/editor) integration page covers the per-editor wiring for VSCode, Neovim, JetBrains, Sublime Text, Emacs, and Helix.

## Other Token-Level Formatters

Black and autopep8 pair with *Prose* through the same two-stage shape, with Black requiring `--skip-magic-trailing-comma` so it doesn't re-expand collections that [[collection-layout]] is responsible for. The conflict table above transfers directly, because Black, autopep8, and Ruff all consume `pycodestyle`'s codes.

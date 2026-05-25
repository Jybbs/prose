# Ruff

<Tool slug="ruff" /> is the token-level formatter most commonly paired with *Prose*. *Prose* doesn't need Ruff to run, in that it produces a settled layout from any well-formed Python source, though pairing the two cleanly takes the small Ruff configuration laid out below.

## Recommended Ordering

```bash
ruff format && prose format
```

::: warning Order Matters
When both tools run on the same file, run Ruff first. *Prose*'s alignment math reads the line breaks already on the file, so a later Ruff re-wrap will undo per-line layout decisions and force a third pass.
:::

A second run of `ruff format` against *Prose*'s output is a no-op, because the alignment padding *Prose* introduces lives within the lines Ruff already settled, leaving Ruff with nothing to re-wrap. The pairing is idempotent end-to-end, meaning a developer can re-run `ruff format && prose format` after a manual edit without expecting either tool to thrash.

## Ruff Configuration

A handful of Ruff's `pycodestyle` codes flag whitespace patterns that *Prose*'s alignment rules deliberately introduce, so a clean pairing needs `extend-ignore` to silence them on the Ruff side. Copy this block into the project's `ruff.toml` *(or under `[tool.ruff]` in `pyproject.toml`)*:

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

The pairing compiles into CI as two sequential check steps, with the exit codes gating the workflow. The <Tool slug="github" /> integration page covers the workflow shape end-to-end:

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

Black and autopep8 pair with *Prose* through the same shape, with Black requiring `--skip-magic-trailing-comma` so it doesn't re-expand collections that [[collection-layout]] is responsible for. The conflict table above transfers directly, because Black, autopep8, and Ruff all consume `pycodestyle`'s codes.

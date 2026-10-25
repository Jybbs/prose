---
summary: 'For a project that already runs Ruff, the `extend-ignore` list that keeps Ruff’s `pycodestyle` codes from reporting the whitespace *Prose*’s alignment rules write.'
tagline: 'alongside Ruff'
---

# Ruff

<Tool slug="ruff" /> is the formatter most often run in the same project as *Prose*. *Prose* does not need Ruff, since it produces a settled layout from any well-formed Python source on its own, and a project that already runs Ruff keeps it beside *Prose* through the small Ruff configuration below.

## Ruff Configuration

Several of Ruff's `pycodestyle` codes report whitespace that *Prose*'s alignment rules write on purpose, so running both cleanly means listing those codes under `extend-ignore` in Ruff's config. Copy this block into `ruff.toml` (*or under `[tool.ruff]` in `pyproject.toml`*):

```toml
[lint]
extend-ignore = [
  "COM812",  # trailing commas
  "E203",    # space before `:`
  "E221",    # space before `=`
  "E251",    # spaces around a keyword's `=`
  "E272",    # space before `import` / `as`
  "E501",    # line length
]

[format]
skip-magic-trailing-comma = true
```

Each entry in the block covers one conflict:

| **Code** | **Conflict** | **Reason** |
|---|---|---|
| `COM812` | *The lint adds trailing commas back* | [[strip-trailing-commas]] removes them from multi-line collections and signatures |
| `E203` | *The lint reports whitespace before `:`* | [[align-colons]] writes it in dict literals, annotated assignments, signatures, and docstring `Args:` sections |
| `E221` | *The lint reports multiple spaces before `=`* | [[align-equals]] writes them across consecutive assignments at one indentation |
| `E251` | *The lint reports spaces around a keyword argument's `=`* | [[align-equals]] writes them around each keyword sitting alone on its line in an exploded call |
| `E272` | *The lint reports multiple spaces before `import` or `as`* | [[align-imports]] writes them across `from ... import ...` and `import ... as ...` runs |
| `E501` | *The lint reports lines longer than `line-length`* | Padding a short row out to a wide column can push it past the limit |
| `skip-magic-trailing-comma` | *The formatter explodes a collection whenever it ends in a trailing comma* | [[reflow-collections]] decides collection layout by width and count rather than by a trailing comma |

## Running Both Formatters

A project that runs `ruff format` as well runs it ahead of `prose format`:

```bash
ruff format path/ && prose format path/
```

::: warning Order Matters
Run Ruff first whenever both tools format the same file. A `ruff format` run over *Prose*'s output removes the horizontal padding, collapsing an aligned `=` run or a shared comment column back to single spaces, so a file Ruff formats last loses the alignment *Prose* wrote.
:::

The two formatters also lay out some constructs differently, so a `ruff format` run over *Prose*'s output re-breaks those as well. It rejoins a short dict or signature *Prose* writes one entry per line, and it writes a row of names *Prose* packs one name per line. *Prose* reads Ruff's line breaks and blank lines the way it reads any source, rewriting the ones its rules govern and keeping the rest as Ruff wrote them. A second run of the pair can therefore change a file once more, where `ruff format` adds a blank line or re-breaks a construct that *Prose* then keeps as Ruff wrote it.

## Import Sorting

Ruff's import-sorting rules (*the isort `I` category, run by `ruff check` rather than `ruff format`*) overlap with *Prose* differently from the formatter codes above. *Prose* orders imports itself, grouping each block into bare imports, then external `from` imports, then local-package imports, with one blank line between groups, which is not isort's layout. A project that runs `ruff check` should leave the `I` rules unselected so the two tools do not rewrite the same import block to different orders.

## In CI

`ruff format --check` reports every row *Prose* pads as a pending change, so a job checks the layout through `prose check` and runs Ruff's linter beside it, either non-zero exit failing the job. The <Tool slug="github" /> integration page covers the whole workflow, the annotation formats, and the SARIF upload:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff check .
- run: prose check .
```

## In an Editor

For format-on-save, chain both commands in the save hook, Ruff first:

```bash
ruff format ${file} && prose format ${file}
```

The [**Editor**](/integrations/editor) page covers the setup for every editor the run-on-save widget documents.

## Other Token-Level Formatters

Black and autopep8 run with *Prose* the same way, with Black needing `--skip-magic-trailing-comma` so it does not explode collections that [[reflow-collections]] lays out. The conflict table above applies as written, because Black, autopep8, and Ruff all use `pycodestyle`'s codes.

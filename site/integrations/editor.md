# Editor

*Prose* shells out cleanly from any editor that supports run-on-save. Two surfaces cover the common cases. For run-on-save rewriting, `prose format <file>` writes to disk. For editors that consume structured diagnostics, `prose check --output-format json --stdin` emits one [**Ruff-shaped**](https://docs.astral.sh/ruff/configuration/#output-format) record per line and stays out of the filesystem.

## Run on Save

Each editor wires the binary differently, wherein the shape is identical at every site, invoking `prose format ${file}` after every save.

<EditorRunOnSave />

The [**`emeraldwalk.RunOnSave`**](https://marketplace.visualstudio.com/items?itemName=emeraldwalk.RunOnSave) extension watches for save events and invokes the command on every match. For Neovim, `silent!` suppresses the shell-out prompt and any non-zero exit from blocking subsequent autocommands. The PyCharm *File Watchers* plugin invokes the binary on every save, replacing the buffer's content with the formatted output.

## Structured Diagnostics

For editors that consume JSON diagnostics directly, `prose check --output-format json --stdin` emits one record per line:

```bash
prose check --output-format json --stdin < file.py
```

Each record carries `code`, `message`, `filename`, `location`, `end_location`, and *(when an auto-fix applies)* a structured `fix` object describing the replacement and its applicability. The shape mirrors what Ruff and ESLint publish, so editors with LSP-style diagnostic surfaces map the records onto inline squiggles and the `fix` payload drives quick-fix actions. The `code` field carries the [[rule-id]] slug, so the diagnostic surface can group by rule.

::: tip Composes With CI Annotations
The same JSON output drives editor squiggles and CI annotations. The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow-command and SARIF shapes that consume `--output-format json` or its `github` and `sarif` siblings.
:::

For the CLI surface that drives every editor path, see the [**Quick Start**](/guide/quick-start) chapter and the [**CLI Reference**](/reference/cli). For pairing the run-on-save command with [**Ruff**](https://docs.astral.sh/ruff/), see the [**Ruff**](/integrations/ruff) integration page.

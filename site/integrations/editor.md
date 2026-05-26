# Editor

*Prose* shells out cleanly from any editor that supports run-on-save or that consumes structured diagnostics from an external command. The project doesn't ship a dedicated language server, in that the editor's existing LSP-style diagnostic surface *(the same one Ruff, Pylint, and other linters publish into)* renders *Prose*'s JSON output without further plumbing. Two surfaces cover the cases. For run-on-save rewriting, `prose format <file>` writes to disk. For editors that consume structured diagnostics, `prose check --output-format json --stdin` emits one [**Ruff-shaped**](https://docs.astral.sh/ruff/configuration/#output-format) record per line and stays out of the filesystem.

## Run on Save

Each editor wires the binary differently, wherein the shape is identical at every site, invoking `prose format ${file}` after every save.

<EditorRunOnSave />

The widget renders the per-editor snippet for the six editors *Prose* documents directly *(VSCode, Neovim, JetBrains, Sublime Text, Emacs, Helix)*. The VSCode card relies on the [**`emeraldwalk.RunOnSave`**](https://marketplace.visualstudio.com/items?itemName=emeraldwalk.RunOnSave) extension, which watches for save events and invokes the command on every match. The Neovim snippet wraps the shell-out in `silent!`, suppressing both the command prompt and any non-zero exit from blocking subsequent autocommands. The JetBrains snippet uses the *File Watchers* plugin, which invokes the binary on every save and replaces the buffer's content with the formatted output. Any other editor with a "run this command on every save" hook accepts the same `prose format ${file}` shape. The save event is load-bearing across every snippet, because *Prose* reads from disk by default rather than from the editor's in-memory buffer.

## Structured Diagnostics

For editors that consume JSON diagnostics directly, `prose check --output-format json --stdin` emits one record per line:

```bash
prose check --output-format json --stdin < file.py
```

Each record carries `code`, `message`, `filename`, `location`, `end_location`, and *(when an auto-fix applies)* a structured `fix` object describing the replacement and its applicability. The shape mirrors what Ruff and ESLint publish, so editors with LSP-style diagnostic surfaces map the records onto inline squiggles and the `fix` payload drives quick-fix actions. The `code` field carries the [[rule-id]] slug, so the diagnostic surface can group by rule.

Stdin mode reads the buffer contents the editor passes in, which is the right path for diagnostics on an unsaved buffer. Run-on-save rewriting, by contrast, operates on the file already on disk, because the rewriter writes back to that file directly. The two paths are independent, in that a project can wire one, both, or neither.

Both subcommands accept a `-` positional in place of `--stdin`, so `prose format - < file.py` and `prose check - < file.py` read source from stdin without naming the flag. The dash is the conventional shape for run-on-save hooks and pre-commit pipelines.

::: tip Composes With CI Annotations
The same JSON output drives editor squiggles and CI annotations. The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow-command and SARIF shapes that consume `--output-format json` or its `github` and `sarif` siblings.
:::

## Latency and Cadence

*Prose* parses the whole file on every invocation, so the round-trip cost scales with the input size rather than with the size of the edit. For files under a few thousand lines, the cost lands well under a typical save event's budget. The format-on-save cadence is the right one to wire, because format-on-type would invoke the parser on every keystroke and the alignment math reads many lines at once. For an editor with an idle-debounce hook, anchoring `prose check --output-format json --stdin` against a one-second idle delay keeps the diagnostic surface fresh without thrashing.

For the CLI surface that drives every editor path, see the [**Quick Start**](/usage/quick-start) chapter and the [**CLI Reference**](/reference/cli). For pairing the run-on-save command with Ruff, see the [**Ruff**](/integrations/ruff) integration page.

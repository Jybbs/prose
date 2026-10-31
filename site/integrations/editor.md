---
summary: 'Formats on save and reports diagnostics as you type through the language server, or formats on save through a save hook.'
tagline: 'save · LSP'
---

# Editor

*Prose* connects to an editor through a language server or through a command the editor runs. The [**`prose server`**](/reference/cli#prose-server) language server offers the most, formatting on save and publishing diagnostics as you type over the protocol the editor already speaks. An editor without a language-server client can run `prose format <file>` on save and `prose check --output-format json --stdin` for diagnostics, one [**Ruff-shaped**](https://docs.astral.sh/ruff/configuration/#output-format) JSON record per line. The command paths read the file from disk or stdin, whereas the server reads the editor's live buffer.

## Language Server

`prose server` speaks the language-server protocol over stdio, so an editor connects to it the way it connects to `ruff server` or any other language server. The server advertises full-document sync and whole-document formatting, so an editor's format-on-save sends a `textDocument/formatting` request that runs the [**pipeline**](/reference/pipeline-order) over the buffer, and the server publishes diagnostics on every open and every change. It resolves each document's [**configuration**](/reference/configuration) exactly as `prose check` does, searching the file's ancestor directories for a project config, applying any matching [**per-pattern overrides**](/reference/configuration#per-pattern-overrides), and reading a standalone script's own PEP 723 block when no project config governs it. The editor and a `prose check` over the same tree therefore use the same settings.

An editor's language-server client starts the binary with the `server` subcommand and talks to it over stdin and stdout. A generic client configuration names the command and the language:

```json
{
  "command": ["prose", "server"],
  "filetypes": ["python"]
}
```

Neovim's built-in client takes the same settings through `vim.lsp.start`:

```lua
vim.lsp.start({
  name = "prose",
  cmd = { "prose", "server" },
  filetypes = { "python" },
})
```

A rewrite that a second run would change again shows up most often under format-on-save, and the terminal notice for that case never reaches an editor. The server sends it as a warning message instead, once per document per session rather than on every save, naming the rules that disagree and the command that reproduces the result. Where the client supports `window/showDocument`, the message carries a **File a report** action that opens a pre-filled bug report, and where it does not, the message text includes the same URL. `report-unstable-output = false` turns the message off along with the terminal notice, as the [**Configuration**](/reference/configuration#top-level-keys) reference covers.

## Run on Save

Each editor wires the command differently, and every one runs `prose format` on each save, on the saved file's path or, in Helix, on the buffer piped through `-`.

<EditorRunOnSave />

The widget shows the snippet for each editor *Prose* documents. Some of the snippets carry a dependency or a wrapper worth knowing about before pasting:

- The VS Code snippet needs the [**`emeraldwalk.RunOnSave`**](https://marketplace.visualstudio.com/items?itemName=emeraldwalk.RunOnSave) extension, which runs the command on every matching save.
- The Neovim snippet wraps the command in `silent!`, which hides the command prompt and keeps a non-zero exit from stopping later autocommands.
- The JetBrains snippet uses the *File Watchers* plugin, which runs the binary on every save and replaces the buffer with the formatted output.
- The Sublime Text snippet needs the [**`SublimeOnSaveBuild`**](https://packagecontrol.io/packages/SublimeOnSaveBuild) package, which runs the project's build on every save whose file name matches the `filename_filter` the snippet sets.

Any other editor with a run-a-command-on-save hook takes the same `prose format ${file}` command. These hooks hide the [**unstable-output notice**](/reference/cli#unstable-output), which is printed to stderr where a `silent!` wrapper or a File Watcher never shows it, so a run-on-save user sees that notice only in a terminal run or through [`prose server`](/reference/cli#prose-server).

## Structured Diagnostics

An editor that reads JSON diagnostics runs `prose check --output-format json --stdin`, which prints one record per line:

```bash
prose check --output-format json --stdin < file.py
```

A diagnostics panel can group the records by the [[rule-id]] slug in `code`, and the [**Output Formats**](/reference/output-formats#json) reference lists every field and the summary record that closes the stream.

Stdin mode is the path for diagnostics on an unsaved buffer, and run-on-save formatting is independent of it, so a project can wire one, both, or neither.

Both subcommands accept `-` in place of `--stdin`, so `prose format - < file.py` and `prose check - < file.py` read from stdin without the flag.

::: tip Composes With CI Annotations
The findings an editor reads as JSON reach CI through the `github` and `sarif` siblings of `--output-format json`, which the [**GitHub Actions**](/integrations/github-actions) page covers.
:::

## Latency and Cadence

*Prose* parses the whole file on every run, so the time a run takes grows with the file's size rather than with the size of the edit. For a file under a few thousand lines it finishes well within a save hook's latency budget. Format on save rather than on every keystroke, because a save hook's `prose format ${file}` reads the file from disk rather than from the editor's buffer, and a format-on-type hook would parse the file on each key while the alignment rules read many lines at once. An editor with an idle hook can run `prose check --output-format json --stdin` after about a second of no typing, which keeps diagnostics current without running the formatter constantly.

The [**Quick Start**](/guide/quick-start) chapter and the [**CLI Reference**](/reference/cli) cover the commands every editor path runs. The [**Ruff**](/integrations/ruff) page covers running Ruff's formatter in the same save hook.

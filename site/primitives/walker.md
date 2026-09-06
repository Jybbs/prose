---
consumedBy: [cli]
consumes: [source]
layer: analysis
stability: internal
summary: "Ignore-aware filesystem walker yielding every `.py`, `.pyi`, and `.pyw` source file and every `.ipynb` notebook under the given paths."
tagline: ignore-aware path walker
---

# Walker

<PrimitiveLayout primitive="walker">

*Walker* finds the files the `check` and `format` subcommands run on. Given a list of path arguments, it yields every Python source file *(`.py`, `.pyi`, `.pyw`)* and Jupyter notebook *(`.ipynb`)* under those paths, honoring `.gitignore`, `.ignore`, and the user's global ignore file. The walker wraps the [**`ignore`**](https://docs.rs/ignore/) crate's `Walk`, so it reads gitignore files with the same semantics Ripgrep, fd, and the other tree-walking tools built on that crate share.


## Consumer-Visible Surface

*Walker* lives at `crate/src/walker.rs` and is `pub(crate)`, so this page documents the CLI behavior the function produces rather than a type a downstream caller can call. What a downstream caller sees is the set of files `prose check` and `prose format` operate on, settled before any [[source]] is built or any [[pipeline]] runs.

A downstream consumer reaches the walker through the CLI's path-mode arguments. Stdin mode bypasses the walker entirely, whether invoked through `--stdin` or the `-` positional alias, because the input stream is a single source written straight to stdout.

At `1.0` the discovery hooks open so a downstream can put its own path source in front of the pipeline.

## Ignore Semantics

The walker honors the `.gitignore` at every level of the walked tree alongside the repo-root `.gitignore`, the project-local `.ignore` files the [**`ignore`**](https://docs.rs/ignore/) crate recognizes, and the user's global gitignore *(typically `~/.config/git/ignore`)*. Hidden files and directories are skipped by default, which is the `ignore` crate's own default.

::: warning No Built-In Skip List
There is no built-in skip list, so a directory like `node_modules`, `__pycache__`, `.venv`, or `target` is visited unless a `.gitignore` covers it or its name starts with a dot, and a fresh tree with no `.gitignore` is visited in full from the path roots. In practice the project's `.gitignore` already covers these, so `prose format .` at a repo root visits close to the Python files `git ls-files` would list.

`.prose-ignore` is **not** recognized as a separate ignore file. *Prose* reads `.gitignore` and `.ignore` for project-local exclusions, because a tool-specific ignore file would split the ignore rules across the toolchain.
:::

## How Multi-Root Walks Compose

The walker accepts a slice of input paths and hands it to `ignore::Walk::from_iter`, which adds every path to one `WalkBuilder` through `WalkBuilder::add` and builds a single walk over that root set. Two paths under the same gitignore-controlled tree share the ignore stack, because a `.gitignore` at the common ancestor applies to both. A file reachable from two roots is yielded once, since the walker records each path it has yielded and skips a repeat.

An empty path list yields an empty iterator, because the CLI requires at least one path outside stdin mode.

## Python File Detection

Each yielded entry passes through `PySourceType::try_from_path` *(from `ruff_python_ast`)*, which classifies the file by its extension. The accepted types are:

- `.py` regular Python source
- `.pyi` type stub
- `.pyw` Windows windowed-Python source
- `.ipynb` Jupyter notebook

Any other extension is skipped, and a file with no extension is skipped too, because `PySourceType` reports no match for it. A file named `script` therefore stays out of the walk even when it contains Python source, since the walker never reads file contents. A symlink whose name carries one of the accepted extensions is not followed, because following it would reach a file outside the tree the caller named, and the CLI prints a note on stderr naming each symlink it passed over.

Extension matching compares the extension string exactly, so `Foo.PY` is skipped on every filesystem, a case-insensitive macOS volume included. Yield order is the underlying `ignore::Walk` order, which is deterministic for one tree on one filesystem but not specified across filesystems, so a downstream that needs the same diagnostic order on every platform should sort the yielded paths before feeding them to the pipeline.

## Parallel Execution

The path-mode CLI collects the walk and then fans out across the file list through [**`rayon`**](https://docs.rs/rayon/), each file getting its own [[source]] construction and [[pipeline]] run, and it reports outcomes in the order the walker yielded them. The walker's iterator is `Send`, so it hands off across rayon worker threads cleanly.

::: tip Single-Threaded for Debugging
Setting `RAYON_NUM_THREADS=1` forces single-threaded execution, which is the right setting when debugging a rule against one file or stepping through the [[pipeline]] in a debugger.
:::

## Re-Using This Primitive

The CLI's path-mode entry point is the walker's consumer. A downstream Rust consumer integrating *Prose* through `Pipeline::run` usually has its own file discovery and builds a [[source]] from each path directly, bypassing the walker. The walker's gitignore semantics often differ from the consumer's own conventions, since a linter wired into a build system already has its list of files and a language server reads the editor's open buffers rather than the disk. The cleanest bypass is to feed the pipeline one [[source]] per path from whatever discovery the host application already runs.

<template #related>

- [[source]] is the value each yielded path is read into.
- [[pipeline]] runs against each constructed *Source*.
- The [**Quick Start**](/usage/quick-start#which-files-get-walked) chapter covers the walk semantics from a user's perspective.

</template>

</PrimitiveLayout>

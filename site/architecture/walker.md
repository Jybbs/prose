---
consumedBy: [cli]
consumes: [source]
layer: analysis
stability: internal
summary: "Ignore-aware filesystem walker yielding every `.py`, `.pyi`, and `.pyw` source file and every `.ipynb` notebook under the given paths."
tagline: ignore-aware path walker
---

# The `walker` Module

<PrimitiveLayout primitive="walker">

The `walker` module finds the files the `check` and `format` subcommands run on. Given a list of path arguments, it yields every Python source file *(`.py`, `.pyi`, `.pyw`)* and Jupyter notebook *(`.ipynb`)* under those paths, honoring `.gitignore`, `.ignore`, and the user's global ignore file. The walker wraps the [**`ignore`**](https://docs.rs/ignore/) crate's `Walk`, so it reads gitignore files with the same semantics Ripgrep, fd, and the other tree-walking tools built on that crate share.


## Consumer-Visible Surface

The `walker` module lives at `crate/src/walker.rs`, and its `walk` function is `pub(crate)`, so this page documents the CLI behavior `walk` produces rather than a type a downstream caller can call. What a downstream caller sees is the set of files `prose check` and `prose format` operate on, settled before any [[source]] is built or any [[pipeline]] runs.

Stdin mode bypasses the walker entirely, whether invoked through `--stdin` or the `-` positional alias, because the input stream is a single source, whose rewrite or diagnostics go straight to stdout.

## Ignore Semantics

The walker honors the `.gitignore` at every level of the walked tree alongside the repo-root `.gitignore`, the project-local `.ignore` files the [**`ignore`**](https://docs.rs/ignore/) crate recognizes, and the user's global gitignore *(typically `~/.config/git/ignore`)*. The crate reads the `.gitignore` files and the global gitignore only inside a Git repository, whereas it reads an `.ignore` file anywhere. Hidden files and directories are skipped by default, which is the `ignore` crate's own default.

::: warning No Built-In Skip List
There is no built-in skip list, so a directory like `node_modules`, `__pycache__`, `.venv`, or `target` is visited unless an ignore file the walk honors covers it or its name starts with a dot, and a tree with no such file is visited in full from the path roots.
:::

## Multi-Root Walks

The walker accepts a slice of input paths and hands it to `ignore::Walk::from_iter`, which adds every path to one `WalkBuilder` through `WalkBuilder::add` and builds a single walk over that root set. Two paths under the same gitignore-controlled tree share the ignore stack, because a `.gitignore` at the common ancestor applies to both. A file reachable from two roots is yielded once, since the walker records each path it has yielded and skips a repeat.

## Python File Detection

Each yielded entry passes through `PySourceType::try_from_path` *(from `ruff_python_ast`)*, which classifies the file by its extension. The accepted types are:

- `.py` regular Python source
- `.pyi` type stub
- `.pyw` Windows windowed-Python source
- `.ipynb` Jupyter notebook

Any other extension is skipped, and a file with no extension is skipped too, because `PySourceType` reports no match for it. A file named `script` beneath a named directory therefore stays out of the walk even when it contains Python source, since the walker never reads file contents. Naming that file on the command line fails the run instead, exiting `4` as a named path that does not exist does, while the other named paths still run.

```console
$ prose check script
error: cannot check `script`, which is not a `.py`, `.pyi`, `.pyw`, or `.ipynb` file
```

A symlink the walk meets beneath a named directory is never followed, because following it could reach a file outside the tree the caller named, and the CLI prints a note on stderr naming each one it passed over whose name carries one of the accepted extensions. A symlink named on the command line is followed, so its own name decides whether its target is formatted or the run fails, and one that resolves to a directory is walked as that directory.

Extension matching compares the extension string exactly, so on every filesystem, a case-insensitive macOS volume included, `Foo.PY` is skipped beneath a named directory and fails the run when named. Yield order is the underlying `ignore::Walk` order, which is deterministic for one tree on one filesystem but not specified across filesystems, so a downstream that needs the same diagnostic order on every platform should sort the yielded paths before feeding them to the pipeline.

## Parallel Execution

The path-mode CLI collects the walk and then fans out across the file list through [**`rayon`**](https://docs.rs/rayon/), each file getting its own [[source]] construction and [[pipeline]] run, and it reports outcomes in the order the walker yielded them. The walker's iterator is `Send`, so it hands off across rayon worker threads cleanly.

::: tip Single-Threaded for Debugging
Setting `RAYON_NUM_THREADS=1` forces single-threaded execution, which is the right setting when debugging a rule against one file or stepping through the [[pipeline]] in a debugger.
:::

## Re-Using This Primitive

The CLI's path-mode entry point is the walker's consumer. A downstream Rust consumer integrating *Prose* through `Pipeline::run` usually has its own file discovery and builds a [[source]] from each path directly, bypassing the walker. The walker's gitignore semantics often differ from the consumer's own conventions, since a linter wired into a build system already has its list of files and a language server reads the editor's open buffers rather than the disk.

<template #related>

- [[source]] is the value each yielded path is read into.
- [[pipeline]] runs against each constructed *Source*.
- The [**Quick Start**](/guide/quick-start#files-a-walk-reaches) chapter covers the walk semantics from a user's perspective.

</template>

</PrimitiveLayout>

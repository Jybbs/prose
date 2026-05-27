# Walker

<PrimitiveLayout primitive="walker">

*Walker* is the recursive path-discovery helper the `check` and `format` subcommands consume. Given a list of path arguments, it yields every Python source file *(`.py`, `.pyi`, `.pyw`)* under those paths, honoring `.gitignore`, `.ignore`, and the user's global ignore file. The walker wraps the [**`ignore`**](https://docs.rs/ignore/) crate's `WalkBuilder`, picking up the same gitignore semantics that Ripgrep, fd, and other tree-walking tools share.


## Consumer-Visible Surface

*Walker* lives at `src/walker.rs` and is itself `pub(crate)`, so the function is documented here for the consumer-visible CLI behavior it shapes rather than as a directly-callable type. The downstream-visible consequence is the set of files `prose check` and `prose format` operate on, with the walk shape settled before any [[source]] construction or [[pipeline]] run.

A downstream consumer in `0.2.x` reaches the walker indirectly through the CLI's path-mode arguments. Stdin mode bypasses the walker entirely, whether invoked through `--stdin` or the `-` positional alias, because the input stream resolves to a single source written straight to stdout.

At `1.0` the discovery hooks open so a downstream can plug its own path source in front of the pipeline.

## Ignore Semantics

The walker honors `.gitignore` files at every level of the walked tree alongside the repo-root `.gitignore`, the project-local `.ignore` files the [**`ignore`**](https://docs.rs/ignore/) crate recognizes, and the user's global gitignore *(typically `~/.config/git/ignore`)*. Hidden files and directories are walked by default rather than skipped, matching Ruff's path-mode behavior.

::: warning No Built-In Skip List
There is no built-in skip list. Directories like `node_modules`, `__pycache__`, `.venv`, or `target` are walked unless a `.gitignore` covers them, meaning a fresh tree with no `.gitignore` walks everything reachable from the path roots. The convention in practice is that the project's `.gitignore` already covers these, leaving `prose format .` against a repo root walking exactly `git ls-files` minus the binary excludes.

`.prose-ignore` is **not** recognized as a separate ignore file. *Prose* defers to `.gitignore` and `.ignore` for project-local exclusions, such that adding a tool-specific ignore file would fragment the ignore surface across the toolchain.
:::

## How Multi-Root Walks Compose

The walker accepts a slice of input paths, where the first path seeds a `WalkBuilder` and subsequent paths add to that builder's root set via `WalkBuilder::add`. Two paths under the same gitignore-controlled tree share the ignore stack, because a `.gitignore` at the common ancestor applies to both walks.

An empty path list yields an empty iterator, because the CLI shape requires at least one path outside stdin mode.

## Python File Detection

Each yielded entry passes through `PySourceType::try_from_path` *(from `ruff_python_ast`)* to ensure the file is Python source. The accepted types are:

- `.py` regular Python source
- `.pyi` type stub
- `.pyw` Windows windowed-Python source

Other extensions are skipped, including Jupyter `.ipynb` notebooks, since *Prose* targets Python source rather than notebook cells. Files whose name carries no extension are also skipped, because `PySourceType` returns no source-type match. A file named `script` without an extension stays out of the walk even if it carries Python source, since the walker doesn't probe file contents.

Extension matching follows the host filesystem's case-sensitivity rules, which means `Foo.PY` walks on a default macOS volume *(case-insensitive)* and is skipped on a typical Linux volume *(case-sensitive)*. Yield order is the underlying `ignore::Walk` order, which is deterministic per-tree on a given filesystem but not specified across filesystems, so a downstream that needs cross-platform-reproducible diagnostic order should sort the yielded paths before feeding them into the pipeline.

## Parallel Execution

The path-mode CLI parallelizes across the yielded file list via [**`rayon`**](https://docs.rs/rayon/), with each file getting its own [[source]] construction and [[pipeline]] run. The walker itself is `Send`, so the iterator hands off across rayon worker threads cleanly.

::: tip Single-Threaded for Debugging
Setting `RAYON_NUM_THREADS=1` forces single-threaded execution, which is the right shape when debugging a rule against a specific file or stepping through the [[pipeline]] in a debugger.
:::

## Re-Using This Primitive

The walker is consumed by the CLI's path-mode entry point. A downstream Rust consumer integrating *Prose* through `Pipeline::run` typically holds its own file-discovery logic and constructs [[source]] values directly from each path, bypassing the walker entirely. The walker's gitignore-based semantics often will not match the consumer's existing conventions, since a linter integrated into a build system already knows which files matter and a language server walks the editor's open buffers rather than the disk. The cleanest bypass is feeding the pipeline a [[source]] per discovered path through whatever discovery already runs in the host application.

<template #related>

- [[source]] is the value the walker's yielded paths feed into.
- [[pipeline]] runs against each constructed *Source*.
- The [**Quick Start**](/usage/quick-start#which-files-get-walked) chapter covers the walk semantics from a user's perspective.

</template>

</PrimitiveLayout>

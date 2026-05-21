# Walker

<PrimitivesComposition :initial-focus="'walker'" />

*Walker* is the recursive path-discovery helper the `check` and `format` subcommands consume. Given a list of path arguments, it yields every Python source file *(`.py`, `.pyi`, `.pyw`)* under those paths, honoring `.gitignore`, `.ignore`, and the user's global ignore file. The walker wraps the [**`ignore`**](https://docs.rs/ignore/) crate's `WalkBuilder`, picking up the same gitignore semantics that Ripgrep, fd, and other tree-walking tools share.


## Public Surface

*Walker* lives at `src/walker.rs` and is `pub(crate)`. The downstream-visible consequence is the set of files `prose check` and `prose format` operate on, with the walk shape settled before any [[source]] construction or [[pipeline]] run.

A downstream consumer in `0.2.x` reaches the walker indirectly through the CLI's path-mode arguments. The `--stdin` shape bypasses the walker entirely, because stdin mode consumes a single source from the input stream and writes to stdout.

The internal API stabilizes toward `1.0` where consumer-implemented file-discovery becomes reachable.

## Ignore Semantics

The walker honors `.gitignore` files at every level of the walked tree alongside the repo-root `.gitignore`, the project-local `.ignore` files the [**`ignore`**](https://docs.rs/ignore/) crate recognizes, and the user's global gitignore *(typically `~/.config/git/ignore`)*. Hidden files and directories are walked by default rather than skipped, matching Ruff's path-mode behavior.

There is **no built-in skip list**. Directories like `node_modules`, `__pycache__`, `.venv`, or `target` are walked unless a `.gitignore` covers them, meaning a fresh tree with no `.gitignore` walks everything reachable from the path roots. The convention in practice is that the project's `.gitignore` already covers these, leaving `prose format .` against a repo root walking exactly `git ls-files` minus the binary excludes.

`.prose-ignore` is **not** recognized as a separate ignore file. *Prose* defers to `.gitignore` and `.ignore` for project-local exclusions, such that adding a tool-specific ignore file would fragment the ignore surface across the toolchain.

## How Multi-Root Walks Compose

The walker accepts a slice of input paths. The first path seeds a `WalkBuilder`, and subsequent paths add to that builder's root set via `WalkBuilder::add`. Two paths under the same gitignore-controlled tree share the ignore stack, in that a `.gitignore` at the common ancestor applies to both walks.

An empty path list yields an empty iterator, because the CLI shape requires at least one path when not in `--stdin` mode.

## Python File Detection

Each yielded entry passes through `PySourceType::try_from_path` *(from `ruff_python_ast`)* to ensure the file is Python source. The accepted types are:

- `.py` regular Python source
- `.pyi` type stub
- `.pyw` Windows windowed-Python source

Other extensions are skipped. Files whose name carries no extension are also skipped, because `PySourceType` returns no source-type match. A file named `script` without an extension stays out of the walk even if it carries Python source, in that the walker doesn't probe file contents.

## Parallel Execution

The path-mode CLI parallelizes across the yielded file list via [**`rayon`**](https://docs.rs/rayon/), with each file getting its own [[source]] construction and [[pipeline]] run. The walker itself is `Send`, so the iterator hands off across rayon worker threads cleanly.

Setting `RAYON_NUM_THREADS=1` forces single-threaded execution, which is the right shape when debugging a rule against a specific file or stepping through the [[pipeline]] in a debugger.

## Re-Using This Primitive

The walker is consumed by the CLI's path-mode entry point. A downstream Rust consumer integrating *prose* through `Pipeline::run` typically holds its own file-discovery logic *(matching the consumer's existing conventions)* and constructs [[source]] values directly from each path, bypassing the walker.

## Related

- [[source]] is the value the walker's yielded paths feed into
- [[pipeline]] runs against each constructed *Source*
- The [**Quick Start**](/guide/quick-start#which-files-get-walked) chapter covers the walk semantics from a user's perspective

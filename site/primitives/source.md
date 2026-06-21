---
consumedBy: [aligner, binding-analysis, colon-targets, docstring, edit, orderer, pipeline, suppression-map, walker]
consumes: []
layer: base
stability: public
summary: "Owned wrapper bundling the original text, AST, tokens, line index, and supporting tables."
tagline: parsed-text wrapper
---

# Source

<PrimitiveLayout primitive="source">

Every rule reads the source file through one shared value. *Source* bundles the original text, the parsed AST, the token stream, the line index, and a table of comment spans into a single owned value the pipeline hands across rule boundaries. Because the text is owned rather than borrowed, *Source* carries no lifetime parameter and is `Send + Sync`, which lets the path-mode CLI parallelize across files through `rayon` without lifetime gymnastics.

## Public Surface

`Source` is fully public in `0.2.x`, so a downstream Rust consumer can construct one, walk the AST, query offsets, and reparse after mutating the text without needing to reach inside the crate.

### Construction

The constructors cover the common shapes:

1. `Source::from_path(path) -> Result<Self, SourceError>` reads the file at `path`, parses it as Python, and returns the wrapped value. The on-disk filename is preserved for diagnostic emission. The parser is `ruff_python_parser` at the pinned crate tag, so a downstream that already depends on the same `ruff_*` workspace sees an AST whose types match its own.
2. `Source::from_str(text: &str) -> Result<Self, ParseError>` parses an in-memory string, returning a *Source* whose synthetic filename is `<source>`. Reach for it in stdin mode, language-server buffers, test fixtures, and any other shape where the text exists in memory rather than on disk.

A Python file the parser cannot recover surfaces as `SourceError::Parse(...)` from `from_path` or `ParseError` from `from_str`, with no partial *Source* returned. Syntax-invalid input never produces a half-built *Source*, so the caller always gets either an error or a fully-parsed value.

### Readers

- `text() -> &str` returns the original source text. Every other reader's offsets land in this string.
- `ast() -> &ModModule` returns the parsed AST root. The wrapping *Source* owns the parse, so the AST is borrow-stable for the value's lifetime.
- `tokens() -> &Tokens` returns the token stream. Useful when a rule's question is comment-shaped or trivia-shaped rather than AST-shaped.
- `binding_analysis() -> &BindingAnalysis` returns the per-source [[binding-analysis]] table, built once during construction.
- `comment_ranges() -> &CommentRanges` returns the comment-range table for trivia walking.

### Offset and Line Helpers

Methods covering the common *"where does this offset land?"* and *"what does the source look like around it?"* questions, grouped by what they answer:

- **Position-from-offset.** `column_of`, `line_column`, `line_index` map a `TextSize` to a column, a `(line, column)` pair, or a 1-indexed line number.
- **Line geometry.** `line_indent_width` reports the indent on the line containing an offset, and `slice` returns the source text covering any `Ranged` value.
- **Line-ending convention.** `newline_str` returns the per-file newline (`\n` or `\r\n`), matching what `from_path` detected at read time.
- **Range and line predicates.** `contains_line_break`, `has_blank_line_before`, `is_line_adjacent` answer line-shaped questions about a range.
- **Comment-aware predicates.** `intersects_comment` reports whether a range crosses a comment span, and `first_token_offset_in_range` finds the first non-trivia token inside a range.

### Mutation

`reparse(text: String) -> Result<Self, ParseError>` returns a fresh *Source* over the mutated text. The pipeline drives this between rules, so each downstream rule reads a settled AST.

### Errors

`SourceError` is `pub` and carries the variants:

1. `SourceError::Io(std::io::Error)` covers every disk failure *(file not found, permission denied, mid-read interruption)*. The wrapped `io::Error` carries the OS-level reason in its `kind()` for callers pattern-matching on the failure mode.
2. `SourceError::Parse(ParseError)` covers every parser failure surfaced by `ruff_python_parser`. The wrapped `ParseError` carries the offset, line, and column of the syntactic problem.

Both variants derive `#[from]` conversions, so `?` propagation lifts the underlying error into the right shape without a manual `map_err`.

## Internal Surface

`suppression_map() -> &SuppressionMap` is `pub(crate)` today, so the in-process *SuppressionMap* type is only reachable from within the crate. Consumers needing to consult suppression state pass through [**`Pipeline::run`**](/primitives/pipeline), which already filters emitted edits and diagnostics. The trait `Rule` that concrete rules implement is `pub(crate)` for the same reason, with both surfaces stabilizing toward `1.0` so downstream consumers can register their own rule types against a stable trait.

## Re-Using This Primitive

*Source* is the value the [[pipeline]] reads, but a downstream is free to construct one on its own. The minimal shape opens a file, walks the AST, and inspects the resulting module without standing up a pipeline at all, fitting test fixtures, AST inspection tools, and custom diagnostic surfaces wherein the full rule loop is not wanted:

```rust
use prose::source::Source;

let source     = Source::from_path("example.py")?;
let module     = source.ast();
let statements = module.body.len();
println!("{statements} top-level statements");
```

A consumer that wants the full rule loop instead builds a [[pipeline]] from a `Config`, hands it the *Source*, and reads the returned text plus diagnostics. The [[pipeline]] primitive page covers the `with_defaults`, `with_filters`, and `for_rule` constructors that drive every shape of consumer pipeline.

A downstream Rust crate consumes *Prose* the same way it consumes the `ruff_*` workspace, through a Git dependency pinned to a release tag:

```toml
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "0.2.3" }
```

The Python wheel exposes the binary rather than the library, so a Python consumer drives the same *Source* indirectly through the CLI surface that the [**Installation**](/usage/installation) chapter walks.

<template #related>

- [[pipeline]] runs the rule loop against a *Source*, reparses between rules, returns the final text and diagnostics.
- [[binding-analysis]] builds against a *Source* during construction and answers binding-shaped questions about every name in every scope.
- [[suppression-map]] is built during *Source* construction and consulted by the pipeline at the edit-emission boundary.
- [[rule-id]] is the handle each rule registers under, consumed by the pipeline's deterministic ordering.

For the rule catalog that runs against the *Source*, the [**Rules**](/rules/) page walks every shipped rule by category.

</template>

</PrimitiveLayout>

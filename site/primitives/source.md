---
consumedBy: [aligner, binding-analysis, colon-targets, docstring, edit, orderer, pipeline, suppression-map, walker, wasm]
consumes: [edit]
layer: base
stability: public
summary: "Owned wrapper bundling the original text, AST, tokens, line index, and supporting tables. Every rule reads through this value."
tagline: parsed-text wrapper
---

# Source

<PrimitiveLayout primitive="source">

Every rule reads the source file through one shared value. *Source* bundles the original text, the parsed AST, the token stream, the line index, and a table of comment spans into a single owned value the pipeline hands across rule boundaries, alongside three tables each built the first time a rule reads it, the [[binding-analysis]], the alignment columns, and the stranded-padding edits. Because the text is owned rather than borrowed, *Source* carries no lifetime parameter and is `Send + Sync`, which lets the path-mode CLI parallelize across files through `rayon` without lifetime gymnastics.

## Public Surface

`Source` is fully public today, so a downstream Rust consumer can construct one, walk the AST, and query offsets without needing to reach inside the crate.

### Construction

The constructors cover the common shapes:

1. `Source::from_path(path) -> Result<Self, SourceError>` reads the file at `path`, parses it as Python, and returns the wrapped value. The on-disk filename is preserved for diagnostic emission. The parser is `ruff_python_parser` at the pinned crate version, so a downstream that already depends on the same `ruff_*` workspace sees an AST whose types match its own.
2. `Source::from_str(text: &str) -> Result<Self, ParseError>` parses an in-memory string, returning a *Source* whose synthetic filename is `<source>`. Reach for it in stdin mode, language-server buffers, test fixtures, and any other shape where the text exists in memory rather than on disk.
3. `Source::parse_named(text: String, name: &str) -> Result<Self, ParseError>` parses an in-memory string the way `from_str` does while carrying `name` the way a file-backed value carries its path, so a diagnostic drawn from text held in memory still names the file it came from. A corpus sweep reading a checkpoint back reaches for it, since the buffer is in memory and the reported defect has to name the file on disk.

*Source* also implements `Clone`, which copies the text, the tree, its token stream, and the comment indexes while leaving each lazy cache to fill on the copy's own first read, so a consumer folding one buffer several ways pays for the parse once and for each derived table only where it reads one.

A Python file the parser cannot recover surfaces as `SourceError::Parse(...)` from `from_path` or `ParseError` from `from_str`, with no partial *Source* returned. Syntax-invalid input never produces a half-built *Source*, so the caller always gets either an error or a fully-parsed value.

### Readers

- `text() -> &str` returns the original source text. Every other reader's offsets land in this string.
- `ast() -> &ModModule` returns the parsed AST root. The wrapping *Source* owns the parse, so the AST is borrow-stable for the value's lifetime.
- `tokens() -> &Tokens` returns the token stream. Useful when a rule's question is comment-shaped or trivia-shaped rather than AST-shaped.
- `token_gaps() -> impl Iterator<Item = (&Token, &Token, TextRange)>` yields each adjacent token pair with the range between them, the trivia the lexer skipped. [[strip-stranded-padding]] reads it for the padding inside a bracket and [[shed-backslash-continuations]] for the gap a continuation sits in.
- `prev_token_end(offset: TextSize) -> TextSize` returns the end of the token before an offset, scanning backward over whitespace and comments. [[space-statements]] reads it for where a header's signature closes and [[shed-redundant-base]] for the position a shed base list reaches back to.
- `binding_analysis() -> &BindingAnalysis` returns the per-source [[binding-analysis]] table, built on the first read and carried across a reparse whenever the rule between leaves every binding standing.
- `comment_ranges() -> &CommentRanges` returns the comment-range table for trivia walking.

### Offset and Line Helpers

Methods covering the common *"where does this offset land?"* and *"what does the source look like around it?"* questions, grouped by what they answer:

- **Position-from-offset.** `column_of`, `line_column`, `line_index` map a `TextSize` to a column, a `(line, column)` pair, or a 1-indexed line number.
- **Line geometry.** `line_indent_width` reports the indent on the line containing an offset, `logical_line_tail` reports the range from an offset to where its logical line closes, a break inside a bracketed construct leaving it open, and `slice` returns the source text covering any `Ranged` value.
- **Line-ending convention.** `newline_str` returns the per-file newline (`\n`, `\r\n`, or `\r`), resolved once when the *Source* is built and reused by every rule that emits a break.
- **Range and line predicates.** `contains_line_break`, `has_blank_line_before`, `consecutive_lines` answer line-shaped questions about a range.
- **Comment-aware predicates.** `intersects_comment` reports whether a range crosses a comment span, and `first_token_offset_in_range` finds the first non-trivia token inside a range.

### Mutation

Between rules the pipeline rebuilds the *Source* over the mutated text, taking the narrowest rebuild those edits allow. `splice_of` finds the innermost statement covering each edit and reparses only those windows, splicing the fresh statements and tokens into the tree and token stream the value already holds and sliding every range past the edits by the delta they describe. Across the standard library the statements a batch edits hold about a quarter of the bytes their modules carry, and a third of the rebuilds decline below and put a whole-file parse back on top of that floor.

`reparse_carrying(text: String, cell_offsets: CellOffsets) -> Result<Self, ParseError>` is the whole-file path beneath it, returning a fresh *Source* over the mutated text and carrying a notebook's cell boundaries forward across the rule. A splice hands the work down to it wherever it declines, which covers an edit no single statement contains, a window whose new text does not parse or lands as more than the one statement filling it, a window whose closing indent moved, an edit writing text no window reads, and every notebook. Both are `pub(crate)`, leaving reparsing inside the crate, and both yield a value equal to a parse of the same text, an equality a debug build asserts after every splice.

Either path then hands the binding table the previous *Source* built to the new one through `inherit`, every offset moved through the `SourceMap` of the applied edits. A rule declares whether its edits leave every binding standing, so one that does hands the table over, whereas a table one of whose offsets an edit replaced is left for the next read to rebuild, as are the layout forecasts behind every rule.

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

```toml-vue
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "{{ $frontmatter.proseVersion }}" }
```

The default `native` feature carries the command line, the cache, the language server, and the file walker. Depending with `default-features = false` drops that machinery, leaving the formatting core alone, which also builds for `wasm32-unknown-unknown`.

The Python wheel exposes the binary rather than the library, so a Python consumer drives the same *Source* indirectly through the CLI surface that the [**Installation**](/usage/installation) chapter walks.

<template #related>

- [[pipeline]] runs the rule loop against a *Source*, reparses between rules, returns the final text and diagnostics.
- [[binding-analysis]] builds against a *Source* on its first read, carries across a reparse where the rule between keeps every binding, and answers binding-shaped questions about every name in every scope.
- [[suppression-map]] is built during *Source* construction and consulted by the pipeline at the edit-emission boundary.
- [[rule-id]] is the handle each rule registers under, consumed by the pipeline's deterministic ordering.

For the rule catalog that runs against the *Source*, the [**Rules**](/rules/) page walks every shipped rule by category.

</template>

</PrimitiveLayout>

---
consumedBy: [aligner, binding-analysis, colon-targets, docstring, edit, orderer, pipeline, suppression-map, walker, wasm]
consumes: [edit]
layer: base
stability: public
summary: "Owns the original text, the AST, the token stream, the line index, and the derived tables every rule reads."
tagline: parsed source wrapper
---

# Source

<PrimitiveLayout primitive="source">

Every rule reads the source file through one shared value. *Source* owns the original text, the parsed AST, the token stream, the line index, and a table of comment spans as a single value the pipeline hands from rule to rule. The same value carries the lazily built tables, meaning the [[binding-analysis]], the alignment columns, and the stranded-padding edits, each built the first time a rule reads it. Because the text is owned rather than borrowed, *Source* carries no lifetime parameter and is `Send + Sync`, which lets the path-mode CLI run files in parallel through `rayon` without lifetime workarounds.

## Public Surface

`Source` is fully public today, so a downstream Rust consumer can construct one, read the AST, and query offsets without reaching inside the crate.

### Construction

The constructors cover the common cases:

1. `Source::from_path(path) -> Result<Self, SourceError>` reads the file at `path`, parses it as Python, and returns the wrapped value. The on-disk filename is kept for diagnostic emission. The parser is `ruff_python_parser` at the pinned crate version, so a downstream that already depends on the same `ruff_*` workspace gets an AST whose types match its own.
2. `Source::from_str(text: &str) -> Result<Self, ParseError>` parses an in-memory string, returning a *Source* whose synthetic filename is `<source>`. Use it in stdin mode, language-server buffers, test fixtures, and any other case where the text exists in memory rather than on disk.
3. `Source::parse_named(text: String, name: &str) -> Result<Self, ParseError>` parses an in-memory string the way `from_str` does while carrying `name` the way a file-backed value carries its path, so a diagnostic drawn from text in memory still names the file it came from. A corpus sweep reading a checkpoint back uses it, since the buffer is in memory and the reported defect has to name the file on disk.

*Source* also implements `Clone`, which copies the text, the tree, the token stream, and the comment indexes while leaving each lazy table to fill on the copy's own first read, so a consumer that formats one buffer several ways pays for the parse once and for each derived table only where it reads one.

A Python file the parser cannot recover returns `SourceError::Parse(...)` from `from_path` or `ParseError` from `from_str`, with no partial *Source*. Syntax-invalid input never produces a half-built *Source*, so the caller always gets either an error or a fully parsed value.

### Readers

- `text() -> &str` returns the original source text. Every other reader's offsets index into this string.
- `ast() -> &ModModule` returns the parsed AST root. The wrapping *Source* owns the parse, so the AST borrow stays valid for the value's lifetime.
- `tokens() -> &Tokens` returns the token stream, for a rule whose question is about comments or trivia rather than the AST.
- `token_gaps() -> impl Iterator<Item = (&Token, &Token, TextRange)>` yields each adjacent token pair with the range between them, the trivia the lexer skipped. [[strip-stranded-padding]] reads it for the padding inside a bracket and [[shed-backslash-continuations]] for the gap a continuation sits in. This reader is `pub(crate)` today, so it stays inside the crate.
- `prev_token_end(offset: TextSize) -> TextSize` returns the end of the token before an offset, scanning backward over whitespace and comments. [[space-statements]] reads it for where a header's signature closes and [[shed-redundant-base]] for the position a removed base list reaches back to. This reader is `pub(crate)` today, so it stays inside the crate.
- `binding_analysis() -> &BindingAnalysis` returns the per-source [[binding-analysis]] table, built on the first read and carried across a reparse whenever the rule between leaves every binding in place.
- `comment_ranges() -> &CommentRanges` returns the comment-range table for reading through trivia.

### Offset and Line Helpers

Methods answering the common *"where does this offset sit?"* and *"what does the source look like around it?"* questions, grouped by what they answer:

- **Position-from-offset.** `column_of`, `line_column`, `line_index` map a `TextSize` to a column, a `(line, column)` pair, or a 1-indexed line number.
- **Line geometry.** `line_indent_width` reports the indent on the line containing an offset, `logical_line_tail` reports the range from an offset to where its logical line closes, a break inside a bracketed construct leaving it open, and `slice` returns the source text covering any `Ranged` value.
- **Line-ending convention.** `newline_str` returns the per-file newline (`\n`, `\r\n`, or `\r`), resolved once when the *Source* is built and reused by every rule that writes a break.
- **Range and line predicates.** `contains_line_break`, `has_blank_line_before`, `consecutive_lines` answer line questions about a range.
- **Comment-aware predicates.** `intersects_comment` reports whether a range crosses a comment span, and `first_token_offset_in_range` finds the first non-trivia token inside a range.

### Mutation

Between rules the pipeline rebuilds the *Source* over the text a rule's edits mutated, taking the narrowest rebuild those edits allow. `splice_of` finds the innermost statement covering each edit and reparses only those windows, splicing the fresh statements and tokens into the tree and token stream the value already has and moving every range past the edits by the delta they describe. Across the standard library the statements a batch edits cover about a quarter of the bytes their modules carry, and a third of the rebuilds decline the splice and pay for a whole-file parse on top of that.

`reparse_carrying(text: String, cell_offsets: CellOffsets) -> Result<Self, ParseError>` is the whole-file path beneath it, returning a fresh *Source* over the mutated text and carrying a notebook's cell boundaries forward across the rule. A splice hands the work down to it wherever it declines, which covers an edit no single statement contains, a window whose new text does not parse or reparses as more than the one statement filling it, a window whose closing indent moved, an edit writing text no window reads, and every notebook. Both are `pub(crate)`, keeping reparsing inside the crate, and both yield a value equal to a parse of the same text, an equality a debug build asserts after every splice.

Through `inherit`, either path then hands the new *Source* the binding table the previous one built, every offset moved through the `SourceMap` of the applied edits. A rule declares whether its edits leave every binding in place, and one that does hands the table over. Where an edit replaced one of the table's offsets, the table is left for the next read to rebuild instead, as are the layout forecasts behind every rule.

### Errors

`SourceError` is `pub` and carries the variants:

1. `SourceError::Io(std::io::Error)` covers every disk failure *(file not found, permission denied, mid-read interruption)*. The wrapped `io::Error` carries the OS-level reason in its `kind()` for a caller matching on the failure mode.
2. `SourceError::Parse(ParseError)` covers every parser failure `ruff_python_parser` reports. The wrapped `ParseError` carries the offset, line, and column of the syntax error.
3. `SourceError::Notebook(NotebookError)` covers a `.ipynb` input whose JSON or cell structure the notebook reader rejects.

Every variant derives a `#[from]` conversion, so `?` propagation converts the underlying error into the right variant without a manual `map_err`.

## Internal Surface

`suppression_map() -> &SuppressionMap` is `pub(crate)` today, so the in-process *SuppressionMap* type is reachable only from within the crate. A consumer that needs suppression state goes through [**`Pipeline::run`**](/primitives/pipeline), which already filters emitted edits and diagnostics. The trait `Rule` that concrete rules implement is `pub(crate)` for the same reason, and both stabilize toward `1.0` so downstream consumers can register their own rule types against a stable trait.

## Re-Using This Primitive

*Source* is the value the [[pipeline]] reads, but a downstream is free to construct one on its own. The minimal program opens a file, reads the AST, and inspects the resulting module without standing up a pipeline at all, which fits test fixtures, AST inspection tools, and custom diagnostic output where the full rule loop is unnecessary:

```rust
use prose::source::Source;

let source     = Source::from_path("example.py")?;
let module     = source.ast();
let statements = module.body.len();
println!("{statements} top-level statements");
```

A consumer that runs the full rule loop instead builds a [[pipeline]] from a `Config`, hands it the *Source*, and reads the returned text plus diagnostics. The [[pipeline]] primitive page covers the `with_defaults`, `with_filters`, and `for_rule` constructors that build every kind of consumer pipeline.

A downstream Rust crate depends on *Prose* the same way it depends on the `ruff_*` workspace, through a Git dependency pinned to a release tag:

```toml-vue
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "{{ $frontmatter.proseVersion }}" }
```

The default `native` feature includes the command line, the cache, the language server, and the file walker. Depending with `default-features = false` drops that machinery, leaving the formatting core alone, which also builds for `wasm32-unknown-unknown`.

The Python wheel ships the binary rather than the library, so a Python consumer drives the same *Source* indirectly through the CLI, which the [**Installation**](/usage/installation) chapter covers.

<template #related>

- [[pipeline]] runs the rule loop against a *Source*, reparses between rules, and returns the final text and diagnostics.
- [[binding-analysis]] builds against a *Source* on its first read, carries across a reparse where the rule between keeps every binding, and answers binding questions about every name in every scope.
- [[suppression-map]] is built during *Source* construction and read by the pipeline at the edit-emission boundary.
- [[rule-id]] is the handle each rule registers under, which the pipeline's deterministic ordering reads.

For the rule catalog that runs against the *Source*, the [**Rules**](/rules/) page lists every shipped rule by category.

</template>

</PrimitiveLayout>

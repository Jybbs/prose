# Source

Every rule reads the source file through one shared value. *Source* bundles the original text, the parsed AST, the token stream, the line index, and the comment-range table into a single owned value that the pipeline hands across rule boundaries. Because the text is owned rather than borrowed, *Source* carries no lifetime parameter and can move across thread boundaries, which lets the path-mode CLI parallelize across files without lifetime gymnastics.

<PrimitivesComposition :initial-focus="'source'" />

## Public Surface

`Source` is fully public in `0.2.x`. A downstream Rust consumer can construct one, walk the AST, query offsets, and reparse after mutating the text.

**Construction.** Source is built from disk through `from_path`, which reads the file, parses it as Python, and returns the wrapped value. The parser is `ruff_python_parser` at the pinned crate tag, so a downstream that already depends on the same `ruff_*` workspace sees an AST whose types match its own.

**Readers.**

- `text() -> &str` returns the original source text. Every other reader's offsets land in this string.
- `ast() -> &ModModule` returns the parsed AST root. The wrapping *Source* owns the parse, so the AST is borrow-stable for the value's lifetime.
- `tokens() -> &Tokens` returns the token stream. Useful when a rule's question is comment-shaped or trivia-shaped rather than AST-shaped.
- `binding_analysis() -> &BindingAnalysis` returns the per-source [[binding-analysis]] table, built once during construction.
- `comment_ranges() -> &CommentRanges` returns the comment-range table for trivia walking.

**Trivia helpers.** A handful of methods cover the common "find a position" questions: `column_of`, `line_column`, `line_index`, `line_indent_width`, `newline_str`, `slice`, plus the predicates `contains_line_break`, `has_blank_line_before`, `intersects_comment`, `is_line_adjacent`, and `first_token_offset_in_range`.

**Mutation.** `reparse(text: String) -> Result<Self, ParseError>` returns a fresh *Source* over the mutated text. The pipeline drives this between rules, so each downstream rule reads a settled AST.

**Errors.** `SourceError` is `pub` and distinguishes IO failures from parse failures, so a caller can surface "could not read" and "could not parse" with the precision the user expects.

## Internal Surface

`suppression_map() -> &SuppressionMap` is `pub(crate)` today, so the in-process *SuppressionMap* type is only reachable from within the crate. Consumers needing to consult suppression state pass through [**`Pipeline::run`**](/primitives/pipeline), which already filters emitted edits and diagnostics. The trait `Rule` that concrete rules implement is `pub(crate)` for the same reason. Both surfaces stabilize toward `1.0`.

## Re-Using This Primitive

A consumer that wants one rule's edits without the surrounding pipeline machinery reaches for the rule struct through the registry. The standard path is to build a [[pipeline]] from a `Config`, hand it a *Source*, and read the returned text plus diagnostics. The [[pipeline]] primitive page covers the `with_defaults`, `with_filters`, and `for_rule` constructors that drive every shape of consumer pipeline.

A downstream Rust crate consumes *prose* the same way it consumes the `ruff_*` workspace, through a Git dependency pinned to a release tag:

```toml
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "0.2.3" }
```

The Python wheel exposes the binary rather than the library, so a Python consumer drives the same *Source* indirectly through the CLI surface that the [**Installation**](/guide/installation) chapter walks.

## Related

- [[pipeline]] runs the rule loop against a *Source*, reparses between rules, returns the final text and diagnostics.
- [[binding-analysis]] builds against a *Source* during construction and answers binding-shaped questions about every name in every scope.
- [[suppression-map]] is built during *Source* construction and consulted by the pipeline at the edit-emission boundary.
- [[rule-id]] is the handle each rule registers under, consumed by the pipeline's deterministic ordering.

For the rule catalog that runs against the *Source*, the [**Rules Overview**](/rules/) page walks every shipped rule by category.

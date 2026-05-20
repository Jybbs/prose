# Docstring

<DependencyGraph />

*Docstring* is the walker that reaches every PEP 257 docstring in a module. The first body statement of the module, each class, and each function may carry a string literal as a docstring, and the walker hands every such literal to a consumer in source order. Three rules ([[docstring-wrap]], [[multi-line-docstrings]], [[no-single-line-docstrings]]) consume the same walk, so the AST traversal lives once in *Docstring* and each rule supplies a handler that decides what to emit per docstring.


## Public Surface (`0.2.x`)

*Docstring* lives at `src/primitives/docstring.rs` and is `pub(crate)`. The downstream-visible consequence is the rewrites the docstring rules emit through the diagnostic stream.

The internal API stabilizes toward `1.0` where consumer-implemented docstring rules become reachable.

## The PEP 257 Definition

A docstring is **the first body statement of a module, class, or function**, when that statement is a single string literal expression. The walker matches that shape exactly:

- The first statement must be an `ExprStmt` *(an expression-statement, not an assignment or call)*
- The expression must be a `StringLiteral` *(not a concatenated `JoinedStr` or an f-string)*
- The string must be a single-part literal *(implicitly concatenated multi-part literals are skipped)*
- The literal must sit on the first body line *(no leading content on the line, since `def f(): """doc"""` doesn't count)*

The walker recurses through nested classes and functions, so a module with deeply nested defs surfaces every nested docstring in source order.

## Internal Surface

The receiver trait carries one required method:

```rust
pub(crate) trait DocstringHandler {
    fn handle(&mut self, lit: &StringLiteral);
    fn walk(&mut self, source: &Source) where Self: Sized;
}
```

`handle` is called for each discovered docstring literal in source order. `walk(source)` drives the receiver across `source`'s module body and every nested scope.

The shape-helper `DocstringBody { range, text }` carries the body slice between a triple-quoted docstring's opener and closer, paired with the source range that slice covers. Rules consuming the docstring body *(typically for re-wrapping or re-formatting the body text)* receive this through a separate helper.

## How Docstring-Wrap Composes

[[docstring-wrap]] consumes the walker and the body helper together. For each discovered docstring, the rule extracts the body, partitions it into description prose and structured sections *(`Args:`, `Returns:`, `Raises:`)*, and rewraps each part against its configured budget *(`docstring-line-length` for description prose, `code-line-length` for structured sections, or both collapsed to one when `docstring-structured-policy = "docstring-line-length"`)*. The rule emits one [[edit]] per docstring body that needs rewrapping.

## How Multi-Line and Single-Line Rules Compose

[[multi-line-docstrings]] examines each discovered docstring to ensure the triple-quoted opener and closer sit on their own lines. [[no-single-line-docstrings]] expands docstrings that fit on one line into the canonical multi-line shape. Both rules read the literal's source position and emit edits that reshape the quote placement without touching the body text.

## Build Pattern

A rule implementing `DocstringHandler` carries the accumulator state and pushes edits or diagnostics from each `handle` call. After `walk(source)` returns, the accumulator carries the full result, and the rule's `apply` method returns the `Vec<Edit>` from that accumulator.

## Reuse Pattern

Adding a docstring-shaped rule is shaped as *"implement `DocstringHandler`, decide per-docstring what edits to emit, call `walk(source)` from inside `apply`"*. The PEP 257 detection, the nested-scope traversal, and the implicitly-concatenated skip all carry through without re-implementation.

## Related

- [[docstring-wrap]] wraps description prose and structured sections to their budgets
- [[multi-line-docstrings]] enforces own-line quote placement
- [[no-single-line-docstrings]] expands single-line shapes
- [[edit]] is the output shape rules emit per docstring
- [[source]] is the input the walker reads against

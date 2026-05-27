---
stability: internal
---

# Docstring

<PrimitiveLayout primitive="docstring">

*Docstring* is the walker that reaches every PEP 257 docstring in a module. The first body statement of the module, each class, and each function may carry a string literal as a docstring, and the walker hands every such literal to a consumer in source order. Three rules ([[docstring-wrap]], [[multi-line-docstrings]], [[no-single-line-docstrings]]) consume the same walk, so the AST traversal lives once in *Docstring* and each rule supplies a handler that decides what to emit per docstring.


## Public Surface

*Docstring* lives at `src/primitives/docstring.rs` and is `pub(crate)`. The downstream-visible consequence is the rewrites the docstring rules emit through the diagnostic stream.

At `1.0` the trait promotes to `pub`, opening the surface to downstream-implemented docstring rules.

## The PEP 257 Definition

A docstring is **the first body statement of a module, class, or function**, when that statement is a single string literal expression. The walker matches that shape exactly:

1. The first statement must be an `ExprStmt` *(an expression-statement, not an assignment or call)*
2. The expression must be a `StringLiteral` *(not a concatenated `JoinedStr` or an f-string)*
3. The string must be a single-part literal *(implicitly concatenated multi-part literals are skipped)*
4. The literal must sit on the first body line *(no leading content on the line, since `def f(): """doc"""` doesn't count)*

The walker recurses through nested classes and functions, so a module with deeply nested defs surfaces every nested docstring in source order.

## Internal Surface

The receiver trait carries one required method, with `walk` provided:

```rust
pub(crate) trait DocstringHandler {
    fn handle(&mut self, lit: &StringLiteral);

    fn walk(&mut self, source: &Source) where Self: Sized { /* provided */ }
}
```

`handle` is the required per-docstring callback, invoked for each discovered literal in source order. `walk(source)` is the provided driver across `source`'s module body and every nested scope, and a consuming rule never overrides it.

Two `pub(crate)` helpers reach for the docstring body:

1. `triple_quoted_body(source, lit) -> Option<DocstringBody>` returns the body slice between a triple-quoted docstring's opener and closer, paired with the source range the slice covers. Returns `None` for non-triple-quoted literals and for inline shapes like `def f(): """doc"""`.
2. `indent_prefix(source, lit) -> &str` returns the whitespace preceding the docstring on its first line, useful when a rule rewraps the body and needs to re-indent the result.

[[colon-targets]] also reaches into leading docstrings independently when emitting `Args:` members. The seam is deliberate, because the colon walker handles one structured-section context inline rather than standing up its own `DocstringHandler`, so this primitive's surface stays unchanged for rules whose question is *"every docstring in source order"* rather than *"every `name: description` line inside an `Args:` block."*

## How Docstring-Wrap Composes

[[docstring-wrap]] consumes the walker and the body helper together. For each discovered docstring, the rule extracts the body, partitions it into description prose and structured sections *(`Args:`, `Returns:`, `Raises:`)*, and rewraps each part against its configured budget *(`docstring-line-length` for description prose, `code-line-length` for structured sections, or both collapsed to one when `docstring-structured-policy = "docstring-line-length"`)*. The rule emits one [[edit]] per docstring body that needs rewrapping.

## How Multi-Line and Single-Line Rules Compose

[[multi-line-docstrings]] examines each discovered docstring to ensure the triple-quoted opener and closer sit on their own lines. [[no-single-line-docstrings]] expands docstrings that fit on one line into the canonical multi-line shape. Both rules read the literal's source position and emit edits that reshape the quote placement without touching the body text.

## Build Pattern

A rule implementing `DocstringHandler` carries the accumulator state and pushes edits or diagnostics from each `handle` call. After `walk(source)` returns, the accumulator carries the full result, and the rule's `apply` method returns the `Vec<Edit>` from that accumulator. The canonical shape:

```rust
struct MyRule<'src> {
    source : &'src Source,
    edits  : Vec<Edit>,
}

impl<'src> DocstringHandler for MyRule<'src> {
    fn handle(&mut self, lit: &StringLiteral) {
        if let Some(edit) = self.consider(lit) {
            self.edits.push(edit);
        }
    }
}

impl Rule for MyRuleConfig {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = MyRule { source, edits: Vec::new() };
        visitor.walk(source);
        visitor.edits
    }
}
```

`consider` is the rule-specific per-docstring decision, returning `Some(edit)` when the literal needs rewriting and `None` otherwise, and the accumulator pattern carries through every consuming rule without variation.

## Re-Using This Primitive

A new docstring rule implements `DocstringHandler::handle`, deciding per docstring what edits to emit, and calls `walk(source)` from inside `apply`. The PEP 257 detection, the nested-scope traversal, and the implicitly-concatenated skip come for free.

<template #related>

- [[docstring-wrap]] wraps description prose and structured sections to their budgets.
- [[multi-line-docstrings]] enforces own-line quote placement.
- [[no-single-line-docstrings]] expands single-line shapes.
- [[edit]] is the output shape rules emit per docstring.

</template>

</PrimitiveLayout>

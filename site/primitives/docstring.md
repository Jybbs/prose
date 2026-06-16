---
stability: internal
---

# Docstring

<PrimitiveLayout primitive="docstring">

*Docstring* is the walker that reaches every PEP 257 docstring in a module. The first body statement of the module, each class, and each function may carry a string literal as a docstring, and the walker hands every such literal to a consumer in source order. Three rules ([[docstring-wrap]], [[docstring-frame]], [[docstring-expand]]) consume the same walk, so the AST traversal lives once in *Docstring* and each rule supplies a closure that decides what to emit per docstring.


## Public Surface

*Docstring* lives at `src/primitives/docstring/` and is `pub(crate)`. The downstream-visible consequence is the rewrites the docstring rules emit through the diagnostic stream.

At `1.0` the trait promotes to `pub`, opening the surface to downstream-implemented docstring rules.

## The PEP 257 Definition

A docstring is **the first body statement of a module, class, or function**, when that statement is a single string literal expression. The walker matches that shape exactly:

1. The first statement must be an `ExprStmt` *(an expression-statement, not an assignment or call)*
2. The expression must be a `StringLiteral` *(not a concatenated `JoinedStr` or an f-string)*
3. The string must be a single-part literal *(implicitly concatenated multi-part literals are skipped)*
4. The literal must sit on the first body line *(no leading content on the line, since `def f(): """doc"""` doesn't count)*

The walker recurses through nested classes and functions, so a module with deeply nested defs surfaces every nested docstring in source order.

## Internal Surface

A docstring rule reaches the walker through the closure-based helper:

```rust
pub(crate) fn rewrite_docstrings<F>(source: &Source, f: F) -> Vec<Edit>
where
    F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
```

`rewrite_docstrings` drives the walk across `source` and threads each discovered docstring through `f`, which receives the source, the literal, and the running edit buffer. The closure pushes whatever edits the rule needs per docstring, and the helper returns the accumulated `Vec<Edit>`.

The walk itself runs through a module-private receiver trait:

```rust
trait DocstringHandler {
    fn handle(&mut self, lit: &StringLiteral);

    fn walk(&mut self, source: &Source) where Self: Sized { /* provided */ }
}
```

`handle` is the per-docstring callback invoked for each discovered literal in source order. `walk(source)` is the provided driver across `source`'s module body and every nested scope, and a consuming type never overrides it. `rewrite_docstrings` composes against the trait through a private collector, leaving the closure helper as the module's sole walking surface.

The `pub(crate)` helpers reach for the docstring literal and its body:

1. `body_docstring(body) -> Option<&StringLiteral>` returns a body's leading PEP 257 docstring literal, the shared detection point for consumers that already hold a `&[Stmt]` body rather than walking the whole module.
2. `triple_quoted_body(source, lit) -> Option<DocstringBody>` returns the body slice between a triple-quoted docstring's opener and closer, paired with the source range the slice covers. Returns `None` for non-triple-quoted literals and for inline shapes like `def f(): """doc"""`.
3. `indent_prefix(source, lit) -> &str` returns the whitespace preceding the docstring on its first line, useful when a rule rewraps the body and needs to re-indent the result.

[[colon-targets]] finds leading docstrings through `body_docstring`, then scans their `Args:` lines itself when emitting members for colon alignment. The split is deliberate, because the two primitives answer structurally different questions. *Docstring* surfaces entry names and the byte range a reorder would carry along, whereas *Colon-Targets* surfaces each line's `:`-position for the aligner's padding math. Two views of the same source, each shaped for its consumer.

## Section-Parsing Surface

A second layer of `pub(crate)` helpers parses Title-case-headed docstring sections into their `name: description` entries, for consumers that walk docstring text rather than the AST. The leaf classifiers shape each line:

```rust
pub(crate) fn section_heading(trimmed: &str) -> bool;
pub(crate) fn entry_description_col(trimmed: &str) -> Option<usize>;
```

`section_heading` matches a Title-case word or multi-word run with every word capitalized, immediately followed by `:`, so Google's canonical headings (`Args:`, `Attributes:`, `Raises:`, `Returns:`, `Yields:`), Numpy's multi-word headings (`Other Parameters:`, `See Also:`), and project-specific custom headings (`Inputs:`, `Steps:`, `Outputs:`) all qualify. `entry_description_col` returns the character column where an entry's description begins after the `name: ` head, matched against a `\w[\w.]*\s*:\s+\S` shape. List-marker recognition (`-`, `*`, `+`, numeric openers) lives in the shared `LineScanner`, which classifies fences, list items, doctest blocks, reStructuredText field lists, section underlines, Sphinx directives, and their continuations as verbatim passthrough, so a section entry whose description carries a bulleted list or interactive example keeps it attached as part of the entry.

The entry iterator composes those leaves into a section walk:

```rust
pub(crate) fn entry_carrying_sections<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Vec<SectionEntry<'src>>>;

pub(crate) struct SectionEntry<'a> {
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
}
```

`entry_carrying_sections` returns one inner vector per section whose body carries at least one entry-shaped line, with each `SectionEntry` carrying the parameter name and the byte range covering the entry's head line through any attached continuations *(verbatim region, hanging description, list item, fenced code block)*. The walker drops sections whose body is prose-only, since the content-shape check filters them out, and drops any docstring whose body is single-line or non-triple-quoted. Continuation attachment reuses the fence and list-indent state the leaf classifiers expose, so a section entry whose description embeds an indented code block keeps the block attached through any downstream reorder.

## How Alphabetize Composes

[[alphabetize]] consumes the entry iterator when its `docstring-entries` knob is on, which is the default. For each docstring, the rule walks `entry_carrying_sections` and reorders the entries within each section, threading the result through the shared `reorder_text` machinery from [[orderer]], so the no-op case allocates nothing. An entry naming a parameter of the documented signature takes that parameter's position as the rule leaves the signature (*source order for the positional run, sorted for the keyword-only block*), and every other entry sinks below the mirrored ones, alphabetized by name. Module and class docstrings carry no signature, so their sections alphabetize throughout. Each section emits one [[edit]] when its entries arrive out of order, with the edit's range covering the section's entries span and leaving the heading and trailing blank line untouched.

Section headings, blank lines between entries, and verbatim continuations *(indented code blocks, fenced blocks, list items)* stay attached to their parent entries through the move because each `SectionEntry`'s range already covers its continuations, leaving the reorder as a straight permutation of byte slices. The `alphabetize` rule carries `docstring-entries` in the `[rules]` table, defaulting to `true`. Setting `alphabetize = { docstring-entries = false }` keeps the AST-level sorts firing while opting out of the docstring-entry reorder, useful when a project curates entry order to match a narrative rather than the signature alphabet.

## How Docstring-Wrap Composes

[[docstring-wrap]] consumes the walker and the body helper together. For each discovered docstring, the rule extracts the body, partitions it into description prose and structured sections *(`Args:`, `Returns:`, `Raises:`)*, and rewraps each part against its configured budget *(`docstring-line-length` for description prose, `code-line-length` for structured sections, or both collapsed to one when `docstring-structured-policy = "docstring-line-length"`)*. The rule emits one [[edit]] per docstring body that needs rewrapping.

## How Multi-Line and Single-Line Rules Compose

[[docstring-frame]] examines each discovered docstring to ensure the triple-quoted opener and closer sit on their own lines. [[docstring-expand]] expands docstrings that fit on one line into the canonical multi-line shape. Both rules read the literal's source position and emit edits that reshape the quote placement without touching the body text.

## Build Pattern

A rule calls `rewrite_docstrings` from its `apply` method and supplies a closure that decides what to emit per docstring:

```rust
impl Rule for MyRule {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(rewrite_docstrings(source, |source, lit, edits| {
            if let Some(edit) = consider(source, lit) {
                edits.push(edit);
            }
        }))
    }
}
```

`consider` is the rule-specific per-docstring decision, returning `Some(edit)` when the literal needs rewriting and `None` otherwise. Rule-specific configuration closes over `self` inside the closure, so a rule with line budgets, allow-patterns, or other knobs reaches them directly without needing a separate accumulator struct. `singleton_groups` wraps each emitted edit into its own fix group on the way out, matching the `Vec<Vec<Edit>>` shape [[edit]] describes.

## Re-Using This Primitive

A new docstring rule's `apply` body is a single `rewrite_docstrings` call carrying the per-docstring decision as a closure. The PEP 257 detection, the nested-scope traversal, and the implicitly-concatenated skip come for free. A rule that needs the `name: description` entries of every Title-case-headed section additionally reaches for `entry_carrying_sections`, which composes the section-detection leaves into a single pass over a docstring's body and hands back a per-section vector of `SectionEntry` ranges the rule can reorder, rewrap, or inspect. Richer per-walk state closes over the closure's environment, since the receiver trait behind the walk is module-private.

<template #related>

- [[alphabetize]] orders the `name: description` entries within each Title-case-headed section, mirroring the documented signature's parameters.
- [[docstring-wrap]] wraps description prose and structured sections to their budgets.
- [[docstring-frame]] enforces own-line quote placement.
- [[docstring-expand]] expands single-line shapes.
- [[edit]] is the output shape rules emit per docstring.

</template>

</PrimitiveLayout>

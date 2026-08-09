---
consumedBy: [alphabetize, colon-targets, docstring-expand, docstring-frame, docstring-wrap, line-overflow, normalize-literals, restated-types, stack-adjacent-strings]
consumes: [edit, source]
layer: analysis
stability: internal
summary: "PEP 257 walker reaching every module, class, and function docstring in source order."
tagline: PEP 257 docstring walker
---

# Docstring

<PrimitiveLayout primitive="docstring">

*Docstring* is the walker that reaches every PEP 257 docstring in a module. The first body statement of the module, each class, and each function may carry a string literal as a docstring, and the walker hands every such literal to a consumer in source order. Several rules consume the same walk, so the AST traversal lives once in *Docstring* and each rule supplies a closure that decides what to emit per docstring.


## Public Surface

*Docstring* lives at `crate/src/primitives/docstring/` and is `pub(crate)`. The downstream-visible consequence is the rewrites the docstring rules emit through the diagnostic stream.

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
pub(crate) fn rewrite_docstrings<F>(source: &Source, f: F) -> Vec<Vec<Edit>>
where
    F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
```

`rewrite_docstrings` drives the walk across `source` and threads each discovered docstring through `f`, which receives the source, the literal, and the running edit buffer. The closure pushes whatever edits the rule needs per docstring, and the helper returns one fix group per docstring, dropping any whose closure left the buffer empty.

`rewrite_docstrings` is itself one caller of the module's own walking helper, which any consumer needing the definition behind each docstring reaches directly:

```rust
pub(crate) fn walk_docstrings<'src>(
    source: &'src Source,
    f: impl FnMut(Option<&'src Stmt>, &'src StringLiteral),
);
```

`walk_docstrings` hands `f` every discovered literal in source order, paired with the class or function definition whose body opens on it and `None` where the module's own docstring is the one reached.

The walk beneath both helpers runs through a module-private receiver trait:

```rust
trait DocstringHandler<'src> {
    fn handle(&mut self, owner: Option<&'src Stmt>, lit: &'src StringLiteral);

    fn walk(&mut self, source: &'src Source) where Self: Sized { /* provided */ }
}
```

`handle` is the per-docstring callback invoked for each discovered literal in source order, carrying the definition that owns it. `walk(source)` is the provided driver across `source`'s module body and every nested scope, and a consuming type never overrides it. `walk_docstrings` composes against the trait through a private closure wrapper, leaving the two closure helpers as the module's walking surface.

The `pub(crate)` helpers reach for the docstring literal and its body:

1. `body_docstring(body) -> Option<&StringLiteral>` returns a body's leading PEP 257 docstring literal, the shared detection point for consumers that already hold a `&[Stmt]` body rather than walking the whole module.
2. `docstring_slots(body) -> Vec<TextRange>` returns the range of the leading string expression in `body` and in every class and function body nested inside it, ascending by start. The slot is the position a docstring occupies whatever its part count, so an implicitly concatenated expression lands here where `body_docstring` skips it. A rule walking a different surface reads it to tell docstring position from an ordinary literal, which is how [[stack-adjacent-strings]] holds a concatenated run filling the slot, how [[line-overflow]] declines to offer it a break, and how [[normalize-literals]] keeps its quote facet off the frame `docstring-frame` owns.
3. `docstring_body(source, lit) -> Option<DocstringBody>` returns the body slice between a docstring's opener and closer whatever its quote style, paired with the source range the slice covers and a `raw` flag carrying whether the literal took an `r` prefix, which is what decides whether a backslash in the slice escapes the character after it. Returns `None` only for an inline shape like `def f(): "doc"`.
4. `triple_quoted_body(source, lit) -> Option<DocstringBody>` narrows `docstring_body` to the canonical `"""` form, the slice `docstring-expand` and `docstring-wrap` act on once `docstring-frame` has requoted every docstring. Returns `None` for a non-triple-quoted literal.
5. `indent_prefix(source, lit) -> &str` returns the whitespace preceding the docstring on its first line, useful when a rule rewraps the body and needs to re-indent the result.
6. `documented_definitions(source) -> Vec<(&Stmt, &StringLiteral)>` returns every class and function definition whose body opens on a docstring, paired with that literal in source order. The module docstring is absent, since a module owns no definition, which is what a rule reading a docstring against the code beneath it needs.

[[colon-targets]] finds leading docstrings through `body_docstring` and their section entries through `entry_carrying_sections`, reading each entry's recorded `:` offset when emitting members for colon alignment. The split is deliberate, because the two primitives answer structurally different questions. *Docstring* surfaces entry names, the `:` separating each from its description, and the byte range a reorder would carry along, whereas *Colon-Targets* shapes those into the members the aligner's padding math consumes. Two views of the same source, each shaped for its consumer.

## Section-Parsing Surface

A second layer of `pub(crate)` helpers parses Title-case-headed docstring sections into their `name: description` entries, for consumers that walk docstring text rather than the AST. The leaf classifiers shape each line:

```rust
pub(crate) fn section_heading(trimmed: &str) -> Option<&str>;
pub(crate) fn sibling_entry_head(
    indent_chars: usize,
    section_body_indent: usize,
    trimmed: &str,
) -> Option<EntryHead<'_>>;
pub(crate) fn typed_entry_head(trimmed: &str) -> bool;
```

`section_heading` returns the heading a line opens with, read without its trailing `:`, matching a Title-case word or multi-word run with every word capitalized, so Google's canonical headings (`Args:`, `Attributes:`, `Raises:`, `Returns:`, `Yields:`), Numpy's multi-word headings (`Other Parameters:`, `See Also:`), and project-specific custom headings (`Inputs:`, `Steps:`, `Outputs:`) all qualify. `sibling_entry_head` reads a line as the `name: description` head of a sibling to the entry above it, returning an `EntryHead` carrying that name with any `*` or `**` prefix excluded, the byte offset where its description begins, and the offset of the separating `:`, found through the shared `unbracketed_colon`, which skips a colon nested inside a parenthesized type (*`markup (str): a string`*) or a bracketed subscript and reads a walrus `:=` as one operator rather than as the separator. A head opens only at the section body indent, one `INDENT_STEP` past the body indent, so a deeper line returns `None` whatever its shape, leaving it a continuation of the entry above. `typed_entry_head` reports whether a head carries that parenthesized type group, which is what holds a `name (type):` line standing outside any section clear of the description wrap. An empty or whitespace-only paren pair restates no type and does not qualify, so a `name ():` line wraps as the prose it is. List-marker recognition (`-`, `*`, `+`, numeric openers) lives in the shared `LineScanner`, which classifies every structured shape it recognizes, along with their continuations, as verbatim passthrough, so a section entry whose description carries a bulleted list, a table, or an interactive example keeps it attached as part of the entry. A line opening on `{` or `[` joins that set as a bracketed literal, whereas `(` reads as prose because a parenthetical aside takes the same opener. An interpreted-text role closes its name on a backtick rather than on whitespace, so a line opening with one reads as prose and wraps with the paragraph carrying it.

The entry iterator composes those leaves into a section walk:

```rust
pub(crate) fn entry_carrying_sections<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Section<'src>>;

pub(crate) struct Section<'a> {
    pub(crate) entries: Vec<SectionEntry<'a>>,
    pub(crate) heading: &'a str,
}

pub(crate) struct SectionEntry<'a> {
    pub(crate) colon: TextSize,
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
    pub(crate) type_group: Option<TextRange>,
}
```

`entry_carrying_sections` returns one `Section` per section whose body carries at least one entry-shaped line, each holding the heading that opened it beside its entries. Each `SectionEntry` carries the parameter name, the source offset of its head line's separating `:`, the byte range covering the entry's head line through every line attached to it, and the range of the parenthesized type where the head carries one. The walker drops sections whose body is prose-only, since the content-shape check filters them out, and drops any docstring whose body is single-line or non-triple-quoted. Continuation attachment reuses the fence and list-indent state the leaf classifiers expose, so a section entry whose description embeds an indented code block keeps the block attached through any downstream reorder.

## How `alphabetize` Composes

[[alphabetize]] consumes the entry iterator when its `sort-docstring-entries` facet is on, which is the default. For each docstring, the rule walks `entry_carrying_sections` and reorders the entries within each section, threading the result through the shared `reorder_text` machinery from [[orderer]], so the no-op case allocates nothing. An entry naming a parameter of the documented signature takes that parameter's position as the rule leaves the signature (*source order for the positional run, sorted for the keyword-only block*), and every other entry sinks below the mirrored ones, alphabetized by name. Module and class docstrings carry no signature, so their sections alphabetize throughout. Each section emits one [[edit]] when its entries arrive out of order, with the edit's range covering the section's entries span and leaving the heading and trailing blank line untouched.

Section headings, blank lines between entries, and verbatim continuations *(indented code blocks, fenced blocks, list items)* stay attached to their parent entries through the move because each `SectionEntry`'s range already covers its continuations, leaving the reorder as a straight permutation of byte slices. The `alphabetize` rule carries `sort-docstring-entries` in the `[rules]` table, defaulting to `true`. Setting `alphabetize = { sort-docstring-entries = false }` keeps the AST-level sorts firing while opting out of the docstring-entry reorder, useful when a project curates entry order to match a narrative rather than the signature alphabet.

## How `docstring-wrap` Composes

[[docstring-wrap]] consumes the walker and the body helper together. For each discovered docstring, the rule extracts the body, partitions it into description prose and structured sections *(`Args:`, `Returns:`, `Raises:`)*, and rewraps each part against its configured budget *(`docstring-line-length` for description prose, `code-line-length` for structured sections, or both collapsed to one when `docstring-structured-policy = "docstring-line-length"`)*. The rule emits one [[edit]] per docstring body that needs rewrapping.

## How `restated-types` Composes

[[restated-types]] consumes `documented_definitions` and the entry iterator together. For each definition carrying a docstring, the rule reads every entry-bearing section and resolves the parameter-documenting headings against that definition's parameters and `Attributes:` against its class body's annotated fields, reporting each entry whose `type_group` names a type the code already declares. The report anchors on the type group alone rather than on the whole entry, and the rule emits no [[edit]], since deciding which of two disagreeing types is correct needs a reader.

## How Multi-Line and Single-Line Rules Compose

[[docstring-frame]] canonicalizes each discovered docstring to the `"""` frame whatever quotes the source carried, and lands a multi-line opener and closer on their own lines. [[docstring-expand]] expands docstrings that fit on one line into the canonical multi-line shape. Both rules read the literal's source position and emit edits that reshape the quote placement without touching the body text.

## Build Pattern

A rule calls `rewrite_docstrings` from its `apply` method and supplies a closure that decides what to emit per docstring:

```rust
impl Rule for MyRule {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        rewrite_docstrings(source, |source, lit, edits| {
            if let Some(edit) = consider(source, lit) {
                edits.push(edit);
            }
        })
    }
}
```

`consider` is the rule-specific per-docstring decision, returning `Some(edit)` when the literal needs rewriting and `None` otherwise. Rule-specific configuration closes over `self` inside the closure, so a rule with line budgets, allow-patterns, or other facets reaches them directly without needing a separate accumulator struct. `rewrite_docstrings` gathers each docstring's edits into their own fix group, matching the `Vec<Vec<Edit>>` shape [[edit]] describes, so a requote and a reframe on one docstring commit as a single suppressible fix.

## Re-Using This Primitive

A new docstring rule's `apply` body is a single `rewrite_docstrings` call carrying the per-docstring decision as a closure. The PEP 257 detection, the nested-scope traversal, and the implicitly-concatenated skip come for free. A rule that needs the `name: description` entries of every Title-case-headed section additionally reaches for `entry_carrying_sections`, which composes the section-detection leaves into a single pass over a docstring's body and hands back a per-section vector of `SectionEntry` ranges the rule can reorder, rewrap, or inspect. Richer per-walk state closes over the closure's environment, since the receiver trait behind the walk is module-private.

<template #related>

- [[alphabetize]] orders the `name: description` entries within each Title-case-headed section, mirroring the documented signature's parameters.
- [[docstring-wrap]] wraps description prose and structured sections to their budgets.
- [[docstring-frame]] enforces own-line quote placement.
- [[docstring-expand]] expands single-line shapes.
- [[restated-types]] reads each section entry's type group against the definition the docstring documents.
- [[edit]] is the output shape rules emit per docstring.

</template>

</PrimitiveLayout>

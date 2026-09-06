---
consumedBy: [alphabetize-siblings, colon-targets, expand-docstrings, frame-docstrings, line-overflow, normalize-literals, restated-types, stack-adjacent-strings, wrap-docstrings]
consumes: [edit, source]
layer: analysis
stability: internal
summary: "Visits every PEP 257 docstring in a module in source order and hands each one to the rule reading it."
tagline: PEP 257 docstring walker
---

# Docstring

<PrimitiveLayout primitive="docstring">

*Docstring* visits every PEP 257 docstring in a module and hands each one, in source order, to the rule reading it. The first body statement of the module, of each class, and of each function may be a string literal, and that literal is the docstring. Several rules read the same sequence of docstrings, so the AST traversal lives once in *Docstring* and each rule supplies a closure that returns the edits for one docstring.


## Public Surface

*Docstring* lives at `crate/src/primitives/docstring/` and is `pub(crate)`. What a downstream caller sees is the rewrites the docstring rules emit through the diagnostic stream.

At `1.0` the visiting helpers become `pub`, so a downstream crate can write a docstring rule of its own against them.

## The PEP 257 Definition

A docstring is **the first body statement of a module, class, or function**, when that statement is a single string literal expression. The walker matches that form exactly:

1. The first statement must be an `ExprStmt` *(an expression statement, not an assignment or a call)*
2. The expression must be a `StringLiteral` *(not a concatenated `JoinedStr` or an f-string)*
3. The string must be a single-part literal *(an implicitly concatenated multi-part literal is skipped)*
4. The literal must open its own line for its body to be read, so the body helpers below return `None` for `def f(): """doc"""` *(the walker still visits that literal, and a rule that rewrites bodies skips it)*

The walker recurses through nested classes and functions, so a module with deeply nested definitions yields every nested docstring in source order.

## Internal Surface

A docstring rule reaches the walker through the closure-based helper:

```rust
pub(crate) fn rewrite_docstrings<F>(source: &Source, f: F) -> Vec<Vec<Edit>>
where
    F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
```

`rewrite_docstrings` visits every docstring in `source` and calls `f` on each one with the source, the literal, and that docstring's edit buffer. The closure pushes whatever edits the rule needs for that docstring, and the helper returns one fix group per docstring, dropping any docstring whose buffer stays empty.

`rewrite_docstrings` is itself one caller of the module's own visiting helper, which a consumer reads directly when it needs the definition each docstring belongs to:

```rust
pub(crate) fn walk_docstrings<'src>(
    source: &'src Source,
    f: impl FnMut(Option<&'src Stmt>, &'src StringLiteral),
);
```

`walk_docstrings` calls `f` on every docstring literal in source order, paired with the class or function definition whose body opens on it, or with `None` for the module's own docstring.

Beneath both helpers, a private `Walker` visitor drives the traversal, calling `f` on each docstring literal in source order with the definition that owns it, so the two closure helpers are the module's only visiting API.

Alongside the visiting helpers, a further set of `pub(crate)` helpers returns the docstring literal and its body:

1. `body_docstring(body) -> Option<&StringLiteral>` returns a body's leading PEP 257 docstring literal, the shared detection point for a consumer that already has a `&[Stmt]` body rather than visiting the whole module.
2. `docstring_slots(body) -> Vec<TextRange>` returns the range of the leading string expression in `body` and in every class and function body nested inside it, ascending by start. The slot is the position a docstring occupies whatever its part count, so an implicitly concatenated expression appears here where `body_docstring` skips it. A rule reading some other construct uses it to tell a docstring from an ordinary literal, which is how [[stack-adjacent-strings]] leaves a concatenated run that fills the slot as written, how [[line-overflow]] declines to offer it a break, and how [[normalize-literals]] keeps its quote facet off the frame `frame-docstrings` owns.
3. `docstring_body(source, lit) -> Option<DocstringBody>` returns the body slice between a docstring's opener and closer whatever its quote style, paired with the source range the slice covers and a `raw` flag recording whether the literal took an `r` prefix, which controls whether a backslash in the slice escapes the character after it. Returns `None` only for an inline form like `def f(): "doc"`.
4. `triple_quoted_body(source, lit) -> Option<DocstringBody>` narrows `docstring_body` to a triple-quoted literal, `"""` or `'''`, the slice `expand-docstrings` and `wrap-docstrings` act on once `frame-docstrings` has requoted every docstring to `"""`. Returns `None` for a non-triple-quoted or inline literal.
5. `indent_prefix(source, lit) -> &str` returns the whitespace before the docstring on its first line, which a rule that rewraps the body reads to re-indent the result.
6. `documented_definitions(source) -> Vec<(&Stmt, &StringLiteral)>` returns every class and function definition whose body opens on a docstring, paired with that literal, in source order. The module docstring is absent, since a module has no definition, so this is the list a rule reads when it compares a docstring against the code beneath it.

[[colon-targets]] finds docstrings through `walk_docstrings` and their entry runs through the `entry_runs` iterator described below, reading each entry's recorded `:` offset when it builds members for colon alignment. The two primitives stay separate because they answer different questions. *Docstring* returns the entry names, the `:` separating each from its description, and the byte range a reorder would move, whereas *Colon-Targets* turns those into the members the aligner's padding math reads. Two views of the same source, each built for its consumer.

## Section-Parsing Surface

A second layer of `pub(crate)` helpers parses the Title-case-headed sections of a docstring into their `name: description` entries, for a consumer that reads docstring text rather than the AST. The leaf classifiers each read one line:

```rust
pub(crate) fn section_heading(trimmed: &str) -> Option<&str>;
pub(crate) fn sibling_entry_head(
    indent_chars: usize,
    section_body_indent: usize,
    trimmed: &str,
) -> Option<EntryHead<'_>>;
pub(crate) fn typed_entry_head(trimmed: &str) -> bool;
```

`section_heading` returns the heading a line opens with, without its trailing `:`. A heading is a Title-case word or a multi-word run with every word capitalized, so Google's headings (`Args:`, `Attributes:`, `Raises:`, `Returns:`, `Yields:`), Numpy's multi-word headings (`Other Parameters:`, `See Also:`), and a project's own headings (`Inputs:`, `Steps:`, `Outputs:`) all qualify. `sibling_entry_head` reads a line as the `name: description` head of a sibling of the entry above it and returns an `EntryHead` carrying the name without any `*` or `**` prefix, the byte offset where the description begins, and the offset of the separating `:`. That `:` is found through the shared `unbracketed_colon`, which skips a colon nested inside a parenthesized type (*`markup (str): a string`*) or a bracketed subscript and reads a walrus `:=` as one operator rather than as the separator. A head opens only at the section body indent, one `INDENT_STEP` past the body indent, so a deeper line returns `None` whatever it contains and reads as a continuation of the entry above. `typed_entry_head` reports whether a head carries that parenthesized type group, which is what keeps a `name (type):` line outside any section clear of the description wrap. An empty or whitespace-only paren pair restates no type and does not qualify, leaving a `name ():` line to wrap as prose.

List-marker recognition (`-`, `*`, `+`, numeric openers) lives in the shared `LineScanner`, which classifies every structured line it recognizes, along with its continuations, as verbatim passthrough. A section entry whose description carries a bulleted list, a table, or an interactive example therefore keeps it attached as part of the entry. A line opening on `{` or `[` joins that set as a bracketed literal, whereas `(` reads as prose because a parenthetical aside takes the same opener. An interpreted-text role closes its name on a backtick rather than on whitespace, so a line opening with one reads as prose and wraps with the paragraph carrying it.

The entry iterator composes those leaves into one pass over the sections:

```rust
pub(crate) fn entry_carrying_sections<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Section<'src>>;

pub(crate) struct Section<'a> {
    pub(crate) entries: Vec<SectionEntry<'a>>,
    pub(crate) heading: &'a str,
}

pub(crate) fn entry_runs<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Vec<SectionEntry<'src>>>;

pub(crate) struct SectionEntry<'a> {
    pub(crate) colon: TextSize,
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
    pub(crate) type_group: Option<TextRange>,
}
```

`entry_carrying_sections` returns one `Section` per section whose body carries at least one entry-shaped line, each carrying the heading that opened it beside its entries. Each `SectionEntry` carries the parameter name, the source offset of the `:` on its head line, the byte range from the entry's head line through every line attached to it, and the range of the parenthesized type where the head carries one. `entry_runs` returns those same entries plus every contiguous run of type-bearing heads standing at the body indent outside any section, so [[colon-targets]] aligns the wider set from `entry_runs` and [[alphabetize-siblings]] sorts the narrower set from `entry_carrying_sections`. `SectionEntry::column_anchor` narrows a type group to the ones written in the Google `name (type)` form, since a `(` flush against its name documents a call and opens no type column, leaving that entry to join a run on its `:` alone. The pass drops a section whose body is prose only, since no line in it reads as an entry, and drops any docstring whose body is single-line or not triple-quoted. Continuation attachment reuses the fence and list-indent state the leaf classifiers expose, so a section entry whose description embeds an indented code block keeps the block attached through any downstream reorder.

## How `alphabetize-siblings` Composes

[[alphabetize-siblings]] reads the entry iterator when its `sort-docstring-entries` facet is on, which is the default. For each docstring, the rule reads `entry_carrying_sections` and reorders the entries within each section, passing the result through the shared `reorder_text` from [[orderer]], so the no-op case allocates nothing. An entry naming a parameter of the documented signature takes that parameter's position as the rule leaves the signature (*source order for the positional run, sorted for the keyword-only block*), and every other entry sinks below the mirrored ones, alphabetized by name. Module and class docstrings carry no signature, so their sections alphabetize throughout. Each section emits one [[edit]] when its entries are out of order, with the edit's range covering the section's entries and leaving the heading and the trailing blank line as written.

Section headings, blank lines between entries, and verbatim continuations *(indented code blocks, fenced blocks, list items)* stay attached to their entries through the move, because each `SectionEntry`'s range already covers its continuations, so the reorder is a straight permutation of byte slices.

The facet itself lives in the `[rules]` table, carried by `alphabetize-siblings` as `sort-docstring-entries` and defaulting to `true`. Setting `alphabetize-siblings = { sort-docstring-entries = false }` keeps the AST-level sorts running and turns off the docstring-entry reorder, for a project that orders its entries to follow a narrative rather than the signature.

## How `wrap-docstrings` Composes

[[wrap-docstrings]] reads the walker and the body helper together. For each docstring, the rule extracts the body, splits it into description prose and structured sections *(`Args:`, `Returns:`, `Raises:`)*, and rewraps each part to its budget *(`docstring-line-length` for description prose, `code-line-length` for structured sections, or one budget for both when `docstring-structured-policy = "docstring-line-length"`)*. The rule emits one [[edit]] per docstring body that needs rewrapping.

## How `restated-types` Composes

[[restated-types]] reads `documented_definitions` and the entry iterator together. For each definition carrying a docstring, the rule reads every entry-bearing section, resolves the parameter-documenting headings against that definition's parameters and `Attributes:` against its class body's annotated fields, and reports each entry whose `type_group` names a type the code already declares. The report points at the type group alone rather than at the whole entry, and the rule emits no [[edit]], since choosing between two disagreeing types needs a reader.

## How Multi-Line and Single-Line Rules Compose

[[frame-docstrings]] requotes each docstring to the `"""` frame whatever quotes the source used, and puts a multi-line opener and closer on their own lines. [[expand-docstrings]] rewrites a docstring that fits on one line into the canonical multi-line form. Both rules read the literal's source position and emit edits that move the quotes without touching the body text.

## Build Pattern

A rule calls `rewrite_docstrings` from its `apply` method and supplies a closure that returns the edits for one docstring:

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

`consider` is the rule's own per-docstring check, returning `Some(edit)` when the literal needs rewriting and `None` otherwise. Rule configuration closes over `self` inside the closure, so a rule with line budgets, allow-patterns, or other facets reads them directly with no separate accumulator struct. `rewrite_docstrings` gathers each docstring's edits into their own fix group, the `Vec<Vec<Edit>>` form [[edit]] describes, so a requote and a reframe on one docstring apply as one suppressible fix.

## Re-Using This Primitive

A new docstring rule's `apply` body is one `rewrite_docstrings` call carrying its per-docstring check as a closure. The PEP 257 detection, the nested-scope traversal, and the implicitly-concatenated skip come with it. A rule that needs the `name: description` entries of every Title-case-headed section also reads `entry_carrying_sections`, which runs the section-detection leaves in one pass over a docstring's body and returns a per-section vector of `SectionEntry` ranges the rule can reorder, rewrap, or inspect. Any richer per-pass state closes over the closure's environment, since the walker behind the traversal is module-private.

<template #related>

- [[alphabetize-siblings]] orders the `name: description` entries within each Title-case-headed section, mirroring the documented signature's parameters.
- [[wrap-docstrings]] wraps description prose and structured sections to their budgets.
- [[frame-docstrings]] requotes to `"""` and puts a multi-line docstring's opener and closer on their own lines.
- [[expand-docstrings]] rewrites a single-line docstring into the multi-line form.
- [[restated-types]] reads each section entry's type group against the definition the docstring documents.
- [[edit]] is the output type a rule emits per docstring.

</template>

</PrimitiveLayout>

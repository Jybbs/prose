---
caption : "Wraps a docstring's description prose to `docstring-line-length` and its Title-case-headed sections to the budget `docstring-structured-policy` selects."
related : [frame-docstrings, expand-docstrings, align-colons]
layout  : doc
---

# wrap-docstrings

<RuleLayout rule="wrap_docstrings">

`wrap-docstrings` wraps the prose inside a docstring to its budget, reading two kinds of text inside one triple-quoted region. The description prose between the opening `"""` and the first section heading wraps as paragraphs at `docstring-line-length`. Every Title-case-headed section below it reads as a code-shaped table, its running text taking the budget `docstring-structured-policy` selects (*`code-line-length`, 88 by default*) so it measures with the code around it, whereas its `name: description` entries wrap to `docstring-line-length` with a hanging indent at the column the description starts on.

The rule reads `docstring-line-length` for the description budget, `code-line-length` for the structured budget, and `docstring-structured-policy` where a project prefers one narrower line across the whole docstring.

A structured block (*a fenced or indented code block, a table, a doctest, a field header*) passes through unwrapped, because its layout carries meaning. An interpreted-text role closes its name on a backtick rather than on whitespace, so a line opening with `:class:` or `:math:` reads as prose and wraps with its paragraph.

Rewrapped prose collapses every interior whitespace run to one space, so the word sequence is identical on every run, whereas a section entry's aligned head stays as written. Its description wraps at the width [[strip-stranded-padding]] settles that head to, where the strip removes the padding before a lone entry's `:`. Section prose wraps one line at a time, first-fit, so a row the rule writes re-wraps to itself. A break that would put a comment marker, an entry head, or a list marker at the start of the next row folds back into the word before it, because the bare `#`, `name:`, or `+` a wrap would leave at the head of the next row reads as the marker it is. A token that reads as a URL or carries an embedded `/` or `-` wraps as one whole word, so an over-budget link overflows the budget intact rather than splitting between its segments.

A backslash ending a line of a non-raw docstring continues it into the next, so the rule resolves that continuation into the join it performs anyway rather than carrying the backslash mid-line as the invalid escape `\ `. A join with no whitespace on either side splices first, so a URL broken across two source lines stays one token. A continuation inside a passthrough block moves with it as written, and a raw docstring has none at all, its backslash being a literal character.

An entry's head line and every line below it that opens no entry of its own read as one paragraph, rewrapped from the description column the settled head leaves, so padding that moves the `:` reflows the whole description rather than stranding the continuation lines. A line whose own form marks it as structure is exempt, so a doctest, a list item, or a bracketed literal under an entry keeps its layout.

The sibling rules [[frame-docstrings]] and [[expand-docstrings]] settle the quoting and the framing before this rule measures anything, and the wrap runs after [[align-colons]] so an entry's budget reflects the column its key was padded to. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each sits.

<template #configuration>

<RuleConfigTable />

The description and structured budgets come from the top-level [**Configuration**](/reference/configuration#top-level-keys) keys, where `docstring-line-length` (*default 76*), `code-line-length` (*default 88*), and `docstring-structured-policy` (*defaulting to `"code-line-length"`*) set the column targets.

</template>

<template #related-after>

For the budget semantics, the [**Docstring Budgets**](/reference/configuration#docstring-budgets) section of the Configuration chapter covers how the description and structured budgets interact.

</template>

</RuleLayout>

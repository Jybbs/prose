---
caption : "Wraps multi-line docstring bodies at the configured measure."
related : [docstring-frame, docstring-expand, align-colons]
layout  : doc
---

# docstring-wrap

<RuleLayout rule="docstring_wrap">

A docstring carries two readings inside one triple-quoted region. The description prose between the opening `"""` and the first section heading reads as paragraphs at `docstring-line-length`. Every Title-case-headed section below it reads as a code-shaped table, its running text taking the budget `docstring-structured-policy` selects (*`code-line-length`, 88 by default*) so it measures with the code around it, whereas its `name: description` entries wrap to `docstring-line-length` with a hanging indent at the description's start column.

The rule reads `docstring-line-length` for the description budget, `code-line-length` for the structured budget, and `docstring-structured-policy` where a project prefers one narrower line across the whole docstring.

A structured shape passes through unwrapped, its layout being load-bearing, covering a fenced or indented code block, a table, a doctest, and a field header. An interpreted-text role closes its name on a backtick rather than on whitespace, so a line opening with `:class:` or `:math:` reads as prose and wraps with its paragraph.

Reflowed prose collapses every interior whitespace run to one space, leaving the word sequence identical on every run, whereas a section entry's aligned head stays verbatim. A token reading as a URL or carrying an embedded `/` or `-` wraps as one atomic word, so an over-budget link overflows the measure intact rather than splitting between its segments.

A backslash ending a line of a non-raw docstring continues it into the next, so the rule resolves that continuation into the join it performs anyway rather than carrying the backslash mid-line as the invalid escape `\ `. A join with no whitespace on either side splices first, so a URL broken across two source lines stays one token. A continuation inside a passthrough region travels with it untouched, and a raw docstring holds none at all.

An entry's head line and every line below it opening no entry of its own read as one paragraph, rewrapped from the head's current description column, so padding that moves the `:` reflows the whole description rather than stranding continuations. A line whose own shape marks it as a structure is exempt, so a doctest, a list item, or a bracketed literal under an entry keeps its layout.

Both sibling docstring rules settle the quoting and the framing before this rule measures anything, and the wrap runs after [[align-colons]] so an entry's budget reflects the column its key was padded to. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each sits.

<template #configuration>

<RuleConfigTable />

Description and structured budgets come from the top-level [**Configuration**](/reference/configuration#top-level-keys) keys: `docstring-line-length` (*default 76*), `code-line-length` (*default 88*), and `docstring-structured-policy` (*defaulting to `"code-line-length"`*) drive the column targets.

</template>

<template #related-after>

For the budget semantics, the [**Docstring Budgets**](/reference/configuration#docstring-budgets) section of the Configuration chapter covers how the description and structured budgets interact.

</template>

</RuleLayout>

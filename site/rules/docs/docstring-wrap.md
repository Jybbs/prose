---
caption : "Wraps multi-line docstring bodies at the configured measure."
related : [docstring-frame, docstring-expand, align-colons]
layout  : doc
---

# docstring-wrap

<RuleLayout rule="docstring_wrap">

A docstring carries two readings inside one triple-quoted region. The description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Every Title-case-headed section that follows reads as a code-shaped table, where its prose lines take the budget `docstring-structured-policy` selects (*`code-line-length`, 88 by default*) so a section's running text runs to the same measure as the code around it, whereas its `name: description` entries wrap to `docstring-line-length` with a hanging indent at the description's start column.

The rule reads `docstring-line-length` for the description budget, `code-line-length` for the structured budget, and `docstring-structured-policy` to override the structured budget when a project prefers a single narrower line across the whole docstring. Code blocks inside the description (*fenced or indented*) are preserved verbatim, since their layout is load-bearing. Every other structured shape passes through unwrapped for the same reason, so a table keeps its columns, a doctest keeps one statement per line, and a field header keeps its type intact rather than collapsing into a paragraph no renderer reads as markup. The examples below carry one case per shape the rule recognizes. An interpreted-text role closes its name on a backtick rather than on whitespace, so a line opening with `:class:` or `:math:` reads as prose and wraps with the paragraph carrying it. The prose the rule reflows collapses every interior whitespace run to one space, leaving the word sequence the wrapper measures identical on every run, whereas a section entry's aligned head stays verbatim. A backslash ending a line of a non-raw docstring continues that line into the next, so the two source lines already carry one logical line of the value. Where the rule reflows that prose it resolves the continuation into the join it performs anyway, rather than carrying the backslash into the paragraph, where the backslash would land mid-line as the invalid escape `\ ` and change the text `help()` renders. A join carrying no whitespace on either side splices before the reflow, so a URL broken across two source lines stays the single atomic token it reads as. A continuation inside a passthrough region travels with the region untouched, and a raw docstring holds no continuations at all, leaving a trailing backslash there a literal character of the value that reflows as an ordinary word. A token that reads as a URL or carries an embedded path `/` or `-` wraps as one atomic word, so an over-budget link overflows the measure intact rather than splitting between its segments. An entry's head line and every line below it from the section body indent onward that opens no entry of its own read as one paragraph, rewrapped from the head's current description column, so padding that moves the `:` reflows the whole description under the new column rather than leaving continuations at the column they were first written to. A line whose own shape marks it as a structure is exempt from that gathering the same way it is anywhere else in the docstring, so a doctest, a list item, or a bracketed literal sitting under an entry keeps its layout whether or not a blank line separates the two. Both sibling docstring rules settle the quoting and the framing before this rule measures anything, and the wrap itself runs after [[align-colons]] so an entry's description budget reflects the column its key was padded to. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each sits.

<template #configuration>

<RuleConfigTable />

Description and structured budgets come from the top-level [**Configuration**](/reference/configuration#top-level-keys) keys: `docstring-line-length` (*default 76*), `code-line-length` (*default 88*), and `docstring-structured-policy` (*defaulting to `"code-line-length"`*) drive the column targets.

</template>

<template #related-after>

For the budget semantics, the [**Docstring Budgets**](/reference/configuration#docstring-budgets) section of the Configuration chapter covers how the description and structured budgets interact.

</template>

</RuleLayout>

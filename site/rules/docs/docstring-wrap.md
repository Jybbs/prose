---
caption : "Wraps multi-line docstring bodies at the configured measure."
related : [docstring-frame, docstring-expand, align-colons]
layout  : doc
---

# docstring-wrap

<RuleLayout rule="docstring_wrap">

A docstring carries two readings inside one triple-quoted region. The description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Every Title-case-headed section that follows reads as a code-shaped table, where its prose lines take the budget `docstring-structured-policy` selects (*`code-line-length`, 88 by default*) so a section's running text runs to the same measure as the code around it, whereas its `name: description` entries wrap to `docstring-line-length` with a hanging indent at the description's start column.

Code blocks inside the description (*fenced or indented*) are preserved verbatim, since their layout is load-bearing. reStructuredText field lists, doctest blocks, section underlines, and Sphinx directives pass through unwrapped for the same reason, so structured markup keeps the shape a renderer reads rather than collapsing into a paragraph. A token that reads as a URL or carries an embedded path `/` or `-` wraps as one atomic word, so an over-budget link overflows the measure intact rather than splitting between its segments. An entry's head line and the continuation lines below it read as one paragraph, rewrapped from the head's current description column, so [[align-colons]] padding that moves the `:` reflows the whole description under the new column rather than leaving continuations at the column they were first written to. The two sibling docstring rules sit upstream of this one: [[docstring-expand]] expands single-line docstrings into the multi-line shape, then [[docstring-frame]] lands the opener and closer on their own lines, and only then does this rule wrap the resulting body.

<template #configuration>

<RuleConfigTable />

Description and structured budgets come from the top-level [**Configuration**](/reference/configuration#top-level-keys) keys: `docstring-line-length` (*default 76*), `code-line-length` (*default 88*), and `docstring-structured-policy` (*defaulting to `"code-line-length"`*) drive the column targets.

</template>

<template #related-after>

For the budget semantics, the [**Docstring Budgets**](/reference/configuration#docstring-budgets) section of the Configuration chapter covers how the description and structured budgets interact.

</template>

</RuleLayout>

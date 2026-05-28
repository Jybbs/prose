---
category : auto-fix
family   : docs
caption  : "wraps multi-line docstring bodies at the configured measure."
related  : [multi-line-docstrings, no-single-line-docstrings]
layout   : doc
---

# docstring-wrap

<RuleLayout rule="docstring_wrap">

A docstring carries two readings inside one triple-quoted region. The description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Every Title-case-headed section that follows reads as a code-shaped table, where the line budget matches the surrounding code's `code-line-length` (*88 by default*) so that argument annotations sit at the same column as the function body's expressions. `docstring-wrap` honors both budgets, wrapping description prose to the narrower line and structured sections to the wider one.

The rule reads `docstring-line-length` for the description budget, `code-line-length` for the structured budget, and `docstring-structured-policy` to override the structured budget when a project prefers a single narrower line across the whole docstring. Code blocks inside the description (*fenced or indented*) are preserved verbatim, since their layout is load-bearing. The two sibling docstring rules sit upstream of this one: [[no-single-line-docstrings]] expands single-line docstrings into the multi-line shape, then [[multi-line-docstrings]] lands the opener and closer on their own lines, and only then does this rule wrap the resulting body.

<template #configuration>

<RuleConfigTable />

Description and structured budgets come from the top-level [**Configuration**](/reference/configuration#top-level-keys) keys: `docstring-line-length` (*default 76*), `code-line-length` (*default 88*), and `docstring-structured-policy` (*defaulting to `"code-line-length"`*) drive the column targets.

</template>

<template #canonical-lead>

Description prose wraps to `docstring-line-length`, with the existing paragraph structure preserved across the rewrap.

</template>

<template #related-after>

For the budget semantics, the [**Docstring Budgets**](/reference/configuration#docstring-budgets) section of the Configuration chapter covers how the description and structured budgets interact.

</template>

</RuleLayout>

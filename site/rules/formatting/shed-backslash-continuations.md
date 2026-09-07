---
caption : "Removes a trailing backslash and rejoins its statement, adding parentheses where the joined line would overflow the budget."
related : [reflow-parentheses, reflow-collections, reflow-imports, line-overflow]
layout  : doc
---

# shed-backslash-continuations

<RuleLayout rule="shed_backslash_continuations">

`shed-backslash-continuations` removes a trailing backslash and rejoins the statement it split, adding parentheses where the joined line would overflow the budget, so a multi-line statement breaks inside brackets everywhere and the reader meets one mechanism rather than a mix of escape characters and brackets. A backslash is the least legible way to split a Python statement, because it pins the continuation to a physical newline rather than to a bracketed group, and every layout rule then has to work around a break the author placed by hand.

What removing the backslash leaves behind depends on where it sits:

1. Where a bracket already spans the break, the backslash does nothing and is removed, leaving the newline for [[reflow-collections]] and its siblings to lay out.
2. A backslash occupying a whole physical line takes that line with it, since nothing else is on it.
3. Everywhere else the statement rejoins onto one line, with a space inserted only where one belongs, so a chain split ahead of `.` or `[` closes up rather than keeping a space before the operator.

A rejoined line that would overflow the budget takes parentheses instead, wrapping the outermost expression the break falls inside and keeping the break where the author put it. Two breaks inside one expression share one pair rather than taking one each. A trailing comment moves onto the rejoined line, and its width counts toward the budget the rejoined line is measured against. Where no expression spans the break, as in an `import` list or an `assert` message, the statement rejoins regardless and [[line-overflow]] reports what no layout can bring within the budget. The one case left untouched is a backslash the lexer folds into a block's indentation, because that backslash sets the indent and no rejoin could keep it.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-backslash-continuations]` directive, which covers every line a continued statement spans.

</template>

</RuleLayout>

---
caption : "Drops a trailing backslash and rejoins its statement, parenthesizing the split where the joined line would overflow."
related : [shed-parentheses, collection-layout, import-layout, line-overflow]
layout  : doc
---

# shed-backslash-continuations

<RuleLayout rule="shed_backslash_continuations">

A trailing backslash is the least legible way to split a Python statement, because the continuation is pinned to a physical newline rather than to a bracketed group, leaving every layout rule to work around a break the author placed by hand. `shed-backslash-continuations` removes the escape and settles the split through brackets instead, so a reader meets one uniform mechanism for a multi-line statement rather than a mix of escape characters and brackets.

Where a bracket already spans the break, the backslash carries nothing and simply goes, leaving the newline for [[collection-layout]] and its siblings to shape. A backslash occupying a whole physical line takes that line with it, since nothing survives its removal. Everywhere else the statement rejoins onto one line, with the separator inserted only where one belongs, so a chain split ahead of `.` or `[` closes up rather than stranding a space before the operator.

A rejoined line that would overflow the budget takes parentheses instead, wrapping the outermost expression the break falls inside and keeping the break where the author put it. Where no expression spans the break, as in an `import` list or an `assert` message, the statement rejoins regardless and [[line-overflow]] flags what no reshape can bring within. The one shape left untouched is a backslash the lexer folds into a block's indentation, which declares that indent and survives no rejoin.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-backslash-continuations]` directive, which holds every line a continued statement spans.

</template>

</RuleLayout>

---
caption : "Explodes a call to one argument per line once its count, its width, or an argument spanning rows calls for it, and rejoins a fractured list where no trigger fires."
related : [alphabetize-siblings, reflow-collections, reflow-signatures, strip-stranded-padding, strip-trailing-commas]
layout  : doc
---

# reflow-calls

<RuleLayout rule="reflow_calls">

`reflow-calls` takes a call whose argument count passes `max-args` and breaks it one argument per line in keyword form, leaving shorter calls inline. The expanded form lays each argument one indent step inside the row its `(` lands on with the closing `)` back at that row's indent, read as the indent the row settles to, so a row opening on the closer of a bracket that opened earlier follows that opener's row rather than the continuation column the source wrote, and a nested eligible call explodes in the same pass.

The pass fires only where every argument is keyword-expressible. A positional argument resolves to its parameter name through the call site's in-module binding, so the exploded form reads `name=value` whatever order the source passed it, and a bare generator expression, a walrus binding, and a `yield` each take a grouping pair. A positional-only prefix, a `*` or `**` unpacking, a callee that does not resolve to a module function, and a `from x import *` anywhere in the module each leave the call inline. The count trigger passes such a call over rather than holding its shape, so a list past `max-args` the pass cannot name closes its fracture and reads on one row the way a list beneath the cap does, leaving `code-line-length` as the only trigger that reaches it.

Every measure reads the column a construct lands at once its parent settles, so a nested call fitting its destination row stays inline, a call following a sibling the pass has just joined or exploded measures the row that sibling leaves it on, and a keyword value answers the budget from the column [[align-equals]] shifts it to, charging the comma closing its row at the position [[alphabetize-siblings]] later seats it at. A call inside a literal [[reflow-collections]] expands, or inside the parameters of a signature [[reflow-signatures]] lays out one per line, waits for that rule, which reshapes it where its entry or parameter lands, and every width reads the row as [[strip-stranded-padding]] leaves it.

A third trigger reads the argument itself. One whose text still spans rows once every closable fracture inside the list shuts explodes the list one argument per line, whatever the count and joined width, so a call carrying a held literal, a nested flush column, or a stacked string run reaches the shape a long call does. A call carrying a single such argument explodes around it, and the held argument travels the way [[reflow-collections]] carries a held member.

Where no trigger fires and the source still spans lines, the pass reads the break. An argument list whose opening `(` ends its line and whose closing `)` opens its own is the flush column the explode path emits, so it holds. Every other break is a fracture and rejoins onto one row, measured across the whole row rather than the list alone, so the rejoin never lands a line the length trigger reopens.

An exploded keyword's value already broken across lines re-indents to the keyword column unless it runs through a multi-line string. A value whose own first row leaves a bracket open seats the rows beneath it one step inside that bracket and drops its closing bracket back to the column the value lands at, so the contents read as sitting inside the bracket rather than beside it.

No trigger reaches a call inside an f-string or t-string replacement field, a spliced line break there being PEP 701 syntax that fails before Python 3.12, leaving an over-wide interpolation to [[line-overflow]].

The rule reshapes layout alone, leaving argument order to [[alphabetize-siblings]], `=` spacing to [[align-equals]], and the trailing comma to [[strip-trailing-commas]].

<template #configuration>

<RuleConfigTable />

</template>

</RuleLayout>

---
caption : "Explodes a call to one keyword argument per line once its argument count passes `max-args`, its width passes `code-line-length`, or an argument spans rows, and rejoins a list broken anywhere else onto one row."
related : [alphabetize-siblings, reflow-collections, reflow-signatures, strip-stranded-padding, strip-trailing-commas]
layout  : doc
---

# reflow-calls

<RuleLayout rule="reflow_calls">

`reflow-calls` breaks a call to one argument per line under any of three triggers, and leaves a shorter call inline:

1. The count trigger, an argument count past `max-args` on a call whose every argument can be written as a keyword.
2. The width trigger, a joined form that overflows `code-line-length` from the column the call sits at.
3. The span trigger, an argument whose text still spans rows once every closable break inside the list has closed.

The count trigger writes each argument as `name=value`, and the exploded form under any trigger puts each argument one indent step inside the row the `(` sits on and the closing `)` back at that row's indent. That indent is the one the row settles to, so a row that opens on the closer of a bracket opened earlier takes the indent of that opener's row rather than the continuation column the source wrote. A nested call that meets a trigger of its own explodes in the same pass.

The count trigger fires only where every argument can be written as a keyword. A positional argument takes its parameter name from the function the call resolves to in the same module, so the exploded form reads `name=value` whatever order the source passed the arguments in. A bare generator expression, a walrus binding, and a `yield` each gain a pair of parentheses around the value. A positional-only prefix, a `*` or `**` unpacking, a callee that does not resolve to a function defined in the module, and a `from x import *` anywhere in the module each leave the call inline. The count trigger skips such a call rather than keeping it as written, so a broken list past `max-args` that the rule cannot name rejoins onto one row the way a broken list under the cap does, and `code-line-length` is the only trigger left that reaches it.

Every width is measured at the column a construct ends up at once its parent settles, reading the row as [[strip-stranded-padding]] leaves it. A nested call that fits the row it ends up on stays inline, a call following a sibling the rule has just joined or exploded is measured on the row that sibling leaves it on, and a keyword value is measured from the column [[align-equals]] shifts it to, with the comma closing its row counted at the position [[alphabetize-siblings]] later puts it at. A call inside a literal that [[reflow-collections]] expands, or inside the parameters of a signature that [[reflow-signatures]] lays out one per line, is left to that rule, which reshapes the call where its entry or parameter ends up.

The span trigger reads the argument itself rather than the list, and that argument explodes the list to one argument per line, whatever the count and the joined width, so a call carrying a literal kept multi-line, a nested list already written one entry per line, or a stacked string run takes the same layout a long call does. A call with a single such argument explodes around it, and that argument moves whole into the exploded form the way [[reflow-collections]] moves a member it keeps as written.

Where no trigger fires and the source still spans lines, the rule reads where the break sits. An argument list whose opening `(` ends its line and whose closing `)` opens its own is the layout the explode writes, so it stays. Every other break rejoins onto one row, measured across the whole row rather than the list alone, so the rejoin never writes a line the width trigger would reopen.

An exploded keyword's value that was already broken across lines re-indents to the keyword's column, unless it runs through a multi-line string. A value whose first row leaves a bracket open puts the rows beneath it one indent step inside that bracket and drops the closing bracket back to the column the value starts at, so the contents read as sitting inside the bracket rather than beside it.

No trigger reaches a call inside an f-string or t-string replacement field, because a line break inside one is PEP 701 syntax that fails to parse before Python 3.12, so an over-wide interpolation is left for [[line-overflow]] to report.

The rule changes layout alone, leaving argument order to [[alphabetize-siblings]], the spacing around `=` to [[align-equals]], and the trailing comma to [[strip-trailing-commas]].

<template #configuration>

<RuleConfigTable />

</template>

</RuleLayout>

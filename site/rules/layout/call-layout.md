---
caption : "Explodes a keyword-expressible call carrying more than the inline-argument cap to one keyword argument per line."
related : [alphabetize, collection-layout, signature-layout, strip-trailing-commas]
layout  : doc
---

# call-layout

<RuleLayout rule="call_layout">

`call-layout` takes a call whose argument count passes `max-args` and breaks it so each argument lands on its own line in keyword form, leaving shorter calls inline. The expanded form lays each argument one indent step past the call with the closing `)` back at the call's own indent, and a nested eligible call explodes in the same pass.

The pass fires only where every argument is keyword-expressible. A positional argument resolves to its parameter name through the call site's in-module binding, so the exploded form reads `name=value` whatever order the source passed it. A bare generator expression, a walrus binding, and a `yield` each take a grouping pair, since none parses after a `name=` prefix. A positional-only prefix, a `*` or `**` unpacking, and a callee that does not resolve to a module function each leave the call inline, as does a `from x import *` anywhere in the module, which can rebind any name a visible `def` appears to define.

Every measure reads the column a construct lands at once its parent settles, so a nested call that fits its destination row stays inline, and a keyword value answers the budget from the column [[align-equals]] shifts it to rather than from the tight `name=value` the source wrote.

Where neither trigger fires and the source still spans lines, the pass reads the break itself. An argument list whose opening `(` ends its line and whose closing `)` opens its own is the flush column the explode path emits, so it holds as written. Every other break is a fracture and rejoins onto one row, the same reading [[collection-layout]] gives a literal. The rejoin measures the whole row rather than the argument list alone, so it never lands a line the length trigger would break open again.

An exploded keyword's value that an earlier pass already broke across lines re-indents to the keyword column rather than splicing in at its old position, unless it runs through a multi-line string.

Neither trigger reaches a call inside an f-string or t-string replacement field, a spliced line break there being PEP 701 syntax that fails before Python 3.12, so an over-wide interpolation is left for [[line-overflow]] to report.

The rule reshapes layout and nothing more, leaving argument order to [[alphabetize]], the `=` spacing to [[align-equals]], and the trailing comma to [[strip-trailing-commas]].

<template #configuration>

<RuleConfigTable />

</template>

</RuleLayout>

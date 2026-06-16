---
caption : "explodes a keyword-expressible call carrying more than the inline-argument cap to one keyword argument per line."
related : [alphabetize, collection-layout, signature-layout, strip-trailing-commas]
layout  : doc
---

# call-layout

<RuleLayout rule="call_layout">

A call carrying enough arguments to read as a wall of positionals folds better one argument per line, where the eye reads each binding on its own and a later edit touches a single line. `call-layout` takes a call whose argument count exceeds `max-inline-args` and breaks it so each argument lands on its own line in keyword form, leaving shorter calls inline.

The pass fires only on a call every argument of which is keyword-expressible. A positional argument resolves to its parameter name through the call site's in-module binding, so the exploded form reads `name=value` whatever order the source passed it. A positional-only prefix, a `*` or `**` unpacking, and a positional call whose callee does not resolve to a module function each leave the call inline, because the rule cannot name those arguments. A `from x import *` clouds resolution the same way, since the wildcard can rebind any name the visible `def` appears to define, leaving the call inline even though the source shows a matching signature. The expanded form lays each argument one indent step past the call, the closing `)` dropping to the call's own indent, and a nested eligible call in an argument value explodes in the same pass.

The rule reshapes layout and nothing more, leaving argument order to [`alphabetize`](/rules/ordering/alphabetize), which runs ahead of it, so disabling alphabetization leaves the exploded arguments in source order. The `=` spacing stays with [`align-equals`](/rules/alignment/align-equals), and whether the last argument carries a trailing comma stays with [`strip-trailing-commas`](/rules/formatting/strip-trailing-commas), which the explode carries through untouched.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `max-inline-args` | positive int \| `false` | `3` | Cap on the argument count an inline call can carry. A call exceeding the cap explodes, so the default `3` explodes a call of four or more arguments. Setting `false` disables the count trigger and leaves every call inline |

</template>

</RuleLayout>

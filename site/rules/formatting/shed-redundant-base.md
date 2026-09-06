---
caption : "Removes an explicit `object` base and the empty parentheses on a class header with no bases."
related : [reflow-parentheses, strip-trailing-commas]
layout  : doc
---

# shed-redundant-base

<RuleLayout rule="shed_redundant_base">

`shed-redundant-base` removes an explicit `object` base and the empty parentheses on a class header with no bases, so `class C(object):` and `class C():` both read `class C:`. Every class on Python 3 inherits from `object` whether or not the header says so, and a header with no bases needs no parentheses, so both forms put tokens between the reader and the class name that say nothing once read.

The two forms are removed the same way because both leave a base list with nothing in it, so the parentheses go with the base rather than standing empty, and a space written between them and the class name goes too. An `object` beside another base is a narrower case, in that the base list still has a member, so only the `object` and the comma joining it to its neighbor go and the parentheses stay for what survives. Two `object` bases written side by side go as one span. A `metaclass=` keyword counts as a surviving member the same way a named base does. Where a base carries a grouping pair of its own, that pair goes with it, though [[reflow-parentheses]] has usually removed such a pair before this rule reads the header.

A base named `object` is only the builtin where the module has not rebound that name ahead of the class, so a module opening `object = LegacyBase` keeps every header written against it. A comment inside the span that would go keeps the header as written too, since removing the span would remove the comment, and that covers a comment beside an `object` base and one inside an otherwise-empty pair alike.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-redundant-base]` directive, which covers every line a wrapped class header spans.

</template>

</RuleLayout>

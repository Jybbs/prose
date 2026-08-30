---
caption : "Drops an explicit `object` base and the empty parentheses a base-less class header carries."
related : [reflow-parentheses, strip-trailing-commas]
layout  : doc
---

# shed-redundant-base

<RuleLayout rule="shed_redundant_base">

Every class on Python 3 inherits from `object` whether or not the header says so, and a header with no bases at all needs no parentheses to hold the nothing it declares. Both forms put tokens between the reader and the class name that carry no information once they are read. `shed-redundant-base` removes them, leaving `class C:` as the bare declaration the longer spellings already meant.

The two forms shed the same way because both leave a base list holding nothing, so the parentheses go with the base rather than standing empty behind it, and a space written between them and the class name goes too. An `object` sitting beside something real is narrower, in that the base list still has a member to carry, so only the `object` and the comma binding it to its neighbor go and the parentheses stay to hold what survives. A `metaclass=` keyword counts as a surviving member the same way a named base does. Where a base carries a grouping pair of its own, that pair goes with it, though [[reflow-parentheses]] has usually cleared such a pair before this rule reads the header.

A base named `object` is only the builtin where the module has not rebound that name ahead of the class, so a module opening `object = LegacyBase` keeps every header written against it. A comment inside the span that would go holds the header as written too, since removing the span would take the comment with it, and that holds for a comment annotating an `object` base and one sitting inside an otherwise-empty pair alike.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[shed-redundant-base]` directive, which holds every line a wrapped class header spans.

</template>

</RuleLayout>

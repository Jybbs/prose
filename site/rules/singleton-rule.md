---
category : auto-fix
family   : formatting
caption  : "strips padding from alignment groups that resolve to a single member."
related  : [align-colons, align-equals, align-imports, match-case-align]
layout   : doc
---

# singleton-rule

<RuleLayout rule="singleton_rule">

An alignment group exists to give the reader's eye a column to drop down. With **two or more members** the column carries information, where each row reads as a row in a table. With **exactly one member** the column becomes a single cell, and padding it to a width that no sibling matches adds visual noise without payoff. `singleton-rule` strips the pre-`:` padding from every `:`-alignment context that resolves to a single member, so a one-key dict, a one-arg signature, or a one-field dataclass reads as **plain code** instead of a one-row table.

The rule operates on the `:`-shaped contexts that [[align-colons]] covers (*dict literals, dataclass and Pydantic fields, function-signature annotations, docstring `Args:` blocks*) plus the single-expression `match`-arm context that [[match-case-align]] covers. Multi-member groups whose `:`s sit on distinct lines pass through this rule untouched, since the colon-alignment surfaces own them. The `=`-alignment from [[align-equals]] and the `import`-keyword alignment from [[align-imports]] carry their own one-member fallbacks and don't need pruning here.

<template #configuration>

<RuleConfigTable />

`singleton-rule` is the cleanup pass for the alignment rules above it, so its only knob is `enabled`. Turning it off leaves one-member alignment contexts as one-row tables *(a one-key dict reading with the same padding a multi-key dict would carry)*, which is rarely what a project wants in practice.

</template>

<template #canonical-lead>

A one-key dict literal drops its pre-`:` padding, reading as a plain key-value pair rather than a one-row table.

</template>

</RuleLayout>

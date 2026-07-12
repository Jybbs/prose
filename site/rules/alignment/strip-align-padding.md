---
caption : "Strips padding that lines up with nothing and settles the gap after a colon, in one-member alignment groups and just inside bracket delimiters."
related : [align-colons, align-equals, align-imports, align-match-case, strip-trailing-commas]
layout  : doc
---

# strip-align-padding

<RuleLayout rule="strip_align_padding">

An alignment group exists to give the reader's eye a column to drop down. With **two or more members** the column carries information, where each row reads as a row in a table. With **exactly one member** the column becomes a single cell, and padding it to a width that no sibling matches adds visual noise without payoff. `strip-align-padding` strips the pre-`:` padding from every `:`-alignment context that resolves to a single member, so a one-key dict, a one-arg signature, or a one-field dataclass reads as **plain code** instead of a one-row table.

The rule operates on the `:`-shaped contexts that [[align-colons]] covers (*dict literals, annotated assignments at any scope, function-signature annotations, Google-style docstring sections*) plus the single-expression `match`-arm context that [[align-match-case]] covers. Multi-member groups whose `:`s sit on distinct lines and open at a shared column pass through this rule untouched, since the colon-alignment surfaces own them. A run whose rows open at differing columns realizes no shared column, so its padding strips here the way a singleton's does. The `=`-alignment from [[align-equals]] and the `import`-keyword alignment from [[align-imports]] carry their own one-member fallbacks and don't need pruning here.

Beyond the pre-`:` gap, the rule settles the run after a colon to one space wherever that colon introduces a value, so a stray `x:   int` reads as `x: int` and a missing space in `x:int` fills to one. A `match`-arm body keeps the spacing [[align-match-case]] gives it, and a docstring entry's description stays as written.

`strip-align-padding` also clears the padding just inside a bracket delimiter, where no alignment rule ever lines anything up. A space run directly after an opening `(`, `[`, or `{`, or directly before its closer, lines up with nothing, so `int(a )` settles to `int(a)` and `[ 1, 2 ]` to `[1, 2]`. Each side strips on its own, and only where the pad shares a line with the content beside it, so a closer on its own line keeps its indent. The braces of an f-string or t-string replacement field are not delimiters this rule touches, wherein a debug `f"{ total = }"` keeps the spaces it echoes into its output. On a `[ 1, 2, ]`, [[strip-trailing-commas]] drops the comma while this rule clears both pads.

<template #configuration>

<RuleConfigTable />

`strip-align-padding` is the cleanup pass for the alignment rules above it, so its only facet is `enabled`. Turning it off leaves one-member alignment contexts as one-row tables *(a one-key dict reading with the same padding a multi-key dict would carry)*, which is rarely what a project wants in practice.

</template>

</RuleLayout>

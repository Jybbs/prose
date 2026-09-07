---
caption : "Removes padding that lines up with nothing, before the `:` of a one-member group and just inside a bracket, and settles the gap after a `:` to one space."
related : [align-colons, align-equals, align-imports, align-match-case, strip-trailing-commas]
layout  : doc
---

# strip-stranded-padding

<RuleLayout rule="strip_stranded_padding">

`strip-stranded-padding` removes the padding before the `:` in every `:`-alignment group that has a single member, so a one-key dict, a one-parameter signature, or a one-field dataclass reads as **plain code** rather than a one-row table. An alignment group with **two or more members** gives the eye a column to drop down, whereas a group with **exactly one member** has no sibling for its padding to line up with, so the padding adds width and nothing else.

The rule reads the `:` contexts [[align-colons]] covers (*dict literals, annotated assignments at any scope, function-signature annotations, Google-style docstring sections*) plus the single-statement `match`-arm context [[align-match-case]] covers. A group of two or more members whose colons sit on separate lines and whose rows start at one shared indent passes through this rule unchanged, since the colon-alignment rules own it. A run whose rows start at differing indents resolves no shared column, so its padding is stripped here the way a single member's is. The `=` alignment of [[align-equals]] and the `import`-keyword alignment of [[align-imports]] handle their own one-member groups and need no stripping here.

Past the gap before the `:`, the rule settles the gap after a colon to one space wherever that colon introduces a value, so a stray `x:   int` becomes `x: int` and `x:int` gains its missing space. A `match`-arm body keeps the spacing [[align-match-case]] writes, and a docstring entry's description stays as written.

`strip-stranded-padding` also removes the padding just inside a bracket delimiter, where no alignment rule ever lines anything up. A run of spaces directly after an opening `(`, `[`, or `{`, or directly before its closer, lines up with nothing, so `int(a )` becomes `int(a)` and `[ 1, 2 ]` becomes `[1, 2]`. Each side is stripped on its own, and only where the padding shares a line with the content beside it, so a closer on its own line keeps its indent. The braces of an f-string or t-string replacement field are not delimiters this rule reads, so a debug `f"{ total = }"` keeps the spaces it echoes into its output. On `[ 1, 2, ]`, [[strip-trailing-commas]] removes the comma while this rule removes both pads.

<template #configuration>

<RuleConfigTable />

`strip-stranded-padding` is the cleanup pass for the alignment rules above it, so its only facet is `enabled`. Turning it off leaves a one-member alignment group as a one-row table *(a one-key dict carrying the same padding a multi-key dict would)*, which is rarely the layout a project chooses.

</template>

</RuleLayout>

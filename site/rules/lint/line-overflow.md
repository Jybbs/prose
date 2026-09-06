---
caption : "Reports a line still over its line budget once no layout rule can shorten it."
related : [reflow-calls, reflow-signatures, reflow-imports]
layout  : doc
---

# line-overflow

<RuleLayout rule="line_overflow">

`line-overflow` reports each line still over its `*-line-length` cap once no layout rule can shorten it, so an unsatisfiable cap shows up as a finding in `prose check` and as a flagged line in the sandbox rather than as a setting that did nothing. The caps are hard limits that every layout rule fits within, so a call, collection, signature, or import that crosses its cap explodes to one entry per line, and an alignment run lays out an over-budget member first and aligns it within the cap after. When no legal layout meets the cap (*a deep indent, a long identifier, a single-name import already at its narrowest, a cap set below what the statement needs*), the narrowest layout stays and this rule names the line.

A line inside an import statement is measured against `import-line-length` and every other line against `code-line-length`. A line a layout rule could still split is left to that rule, so `line-overflow` reports only what no layout rule can shorten. The lines a layout rule can split include an inline call carrying arguments, a multi-element collection, a comma-joined import of either form, a signature carrying parameters, a single-statement match arm, an implicitly concatenated string run outside a docstring slot, and a line of docstring prose. No rule reaches a construct inside an f-string or t-string replacement field, so a line whose only splittable construct sits there is reported here as well. The lint never rewrites, so the diagnostic is reported and the source stays as written.

An overflow that sits inside one string literal with interior whitespace has a reshape even though no rule performs it, because the whitespace gives a legal place to break and adjacent literals inside parentheses join at compile time into the identical value. `line-overflow` carries that parenthesized form as a display-only suggestion, so `prose check` renders the layout and `prose format` never writes it. It stays a suggestion because the break points would become source, where a word inserted near the front reflows every line beneath it and the diff then claims the whole literal changed.

Two cases draw no suggestion, and their findings differ. A literal with no interior whitespace has nowhere legal to break, so a URL, a hash, or a dense regex keeps the report ending *"with no legal reshape"*. A literal that would fit whole one indent below its line needs no break either, because the overflow came from the width ahead of it, so its report stays bare rather than claiming nothing could be done.

<template #configuration>

<RuleConfigTable />

`suggest-string-splits` gates the suggested form alone, not the report. With it off, an over-budget line whose literal could take a break is still reported, and the report omits the *"with no legal reshape"* ending, because a reshape exists there whether or not the finding spells it out.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[line-overflow]` directive. The [**Lengths**](/reference/configuration#lengths) section of the configuration reference states the hard-limit contract on the caps that this rule completes.

</template>

</RuleLayout>

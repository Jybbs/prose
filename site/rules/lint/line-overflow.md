---
caption : "Reports a line still over its line budget once no layout rule can shorten it."
related : [reflow-calls, reflow-collections, reflow-signatures, reflow-imports]
layout  : doc
---

# line-overflow

<RuleLayout rule="line_overflow">

`line-overflow` reports each line still over its `*-line-length` cap once no layout rule can shorten it, so an unsatisfiable cap shows up as a finding in `prose check` and as a flagged line in the sandbox rather than as a setting that did nothing. The caps are hard limits that every layout rule fits within, so a call, collection, signature, or import that crosses its cap explodes to one entry per line, and an alignment run lays out an over-budget member first and aligns it within the cap after. When no legal layout meets the cap (*a deep indent, a long identifier, a single-name import already at its narrowest, a cap set below what the statement needs*), the narrowest layout stays and this rule names the line.

A line inside an import statement is measured against `import-line-length` and every other line against `code-line-length`, its trailing whitespace left out of the width. A row of docstring prose is measured instead against the budget `wrap-docstrings` wraps it to (*`docstring-line-length` for description prose and section entries, the budget `docstring-structured-policy` selects for other section prose*). A row a layout rule will split is left to that rule, so `line-overflow` reports only what the layout rules leave over the cap. The constructs a layout rule splits are:

- A call carrying arguments
- A collection `reflow-collections` can expand (*a dict of one or more entries, or a bracketed list, set, or tuple of two or more*)
- A comma-joined import of either form
- A signature carrying parameters
- A single-statement match arm
- An implicitly concatenated string run outside a docstring slot
- A row of docstring prose `wrap-docstrings` rewraps

Written on one row, a construct leaves that whole row to its rule. Written across rows, it leaves only a row holding two of its parts that the rule separates, such as two arguments, a bracket and the item beside it, or a dict entry's key and value. A construct already laid out one part per row leaves no row to its rule, so a row holding one long argument is reported. A docstring works the same way, in that one still awaiting `frame-docstrings` or `expand-docstrings` leaves every row to that rule, whereas a framed one leaves only the rows `wrap-docstrings` rewraps, so a URL it leaves as written is reported. A signature leaves its row to `reflow-signatures` only where that rule lays it out one parameter per row, so a stub whose `(` through `:` fits the cap while its ` ...` body runs past it is reported. A bare tuple, such as the `name, entry` a comprehension binds, is not among these constructs, because it carries no bracket to break at. A line holding one of them is still reported when:

- Its code fits the cap while an ordinary trailing comment runs past it, because a layout rule measures a row only up to its trailing comment
- A `# prose: skip` holds the rule that would split it, which then leaves the line as written
- The configuration turns that rule off, sets `explode = false` on `reflow-collections`, or sets `split-multi-module = false` on `reflow-imports` for an `import a, b`, and `--select line-overflow` turns every other rule off the same way
- The construct sits inside an f-string or t-string replacement field, which no rule reaches

A line whose code fits once the run of tool pragmas (*`# type:`, `# noqa`*) and `# prose:` directives ending its trailing comment is set aside is not reported at all, since a pragma governs the line it sits on and cannot move off it. An ordinary note ahead of that run still counts toward the width. The lint never rewrites, so the diagnostic is reported and the source stays as written.

An overflow that sits inside one string literal with interior whitespace has a reshape even though no rule performs it, because the whitespace gives a legal place to break and adjacent literals inside parentheses join at compile time into the identical value. `line-overflow` carries that parenthesized form as a display-only suggestion, so `prose check` renders the layout and `prose format` never writes it. It stays a suggestion because the break points would become source, where a word inserted near the front reflows every line beneath it and the diff then claims the whole literal changed.

Each report ends by naming why its line stays over the cap:

| **Ending** | **When** |
|---|---|
| *"with a legal reshape at the string literal"* | A single-part literal crosses the cap, has interior whitespace, and `suggest-string-splits` is on |
| *No ending* | That literal would fit whole one indent below its line, or `suggest-string-splits` is off |
| *"with only its trailing comment past it"* | The line's code fits the cap and only its trailing comment runs past it |
| *"with `reflow-calls` held by a skip"* | A `# prose: skip` holds the rule the ending names over a construct on the line |
| *"with `reflow-calls` off"* | The configuration turns off the rule the ending names, which would split a construct on the line |
| *"with `explode` off on `reflow-collections`"* | The rule runs, but the configuration turns off the setting the ending names, `explode` on `reflow-collections` or `split-multi-module` on `reflow-imports`, which would split a construct on the line |
| *"with no legal reshape"* | Every other reported line |

A literal with no interior whitespace has nowhere legal to break, so a URL, a hash, or a dense regex takes the last ending. A literal that would fit whole one indent below its line needs no break, because the overflow came from the width ahead of it, so its report stays bare rather than claiming nothing could be done.

<template #configuration>

<RuleConfigTable />

`suggest-string-splits` gates the suggested form alone, not the report. With it off, an over-budget line whose literal could take a break is still reported, and the report omits the *"with no legal reshape"* ending, because a reshape exists there whether or not the finding spells it out.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[line-overflow]` directive. The [**Lengths**](/reference/configuration#lengths) section of the configuration reference states the hard-limit contract on the caps that this rule completes.

</template>

</RuleLayout>

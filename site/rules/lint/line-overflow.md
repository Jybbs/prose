---
caption : "Names any line still over its governing cap once no reshape can shorten it."
related : [call-layout, signature-layout, import-layout]
layout  : doc
---

# line-overflow

<RuleLayout rule="line_overflow">

The `*-line-length` caps are hard constraints, and every shaping rule resolves within them, so a call, collection, signature, or import that crosses its cap reshapes to one entry per line, and an alignment run reshapes an over-budget member before aligning. When no legal form satisfies the cap (*a deep indent, a long identifier, a single-name import already at its narrowest, a cap set below what the statement needs*), the narrowest form stands and `line-overflow` names it, so an unsatisfiable cap reads as a finding in `prose check` and as a squiggle in the sandbox rather than as a knob that did nothing.

A line inside an import statement answers to `import-line-length`, every other line to `code-line-length`. A line a layout rule could still split (*an inline call carrying arguments, a multi-element collection, a comma-joined import of either form, a signature carrying parameters, a single-statement match arm, a leading docstring*) is left for that rule, so `line-overflow` surfaces only the remainder no layout rule can shorten. No rule reaches a construct inside an f-string or t-string replacement field, so a line whose only splittable construct sits there surfaces here as well. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

Not every remainder is beyond help. Where the overflow sits inside one string literal whose interior whitespace gives a break somewhere legal to land, a reshape does exist even though no rule performs it, because adjacent string literals inside parentheses join at compile time into the identical value. `line-overflow` names that case and carries the parenthesized form as a display-only suggestion, so `prose check` renders the shape while `prose format` never writes it. The reshape stays a suggestion because its break points become part of the source, where a word inserted near the front reflows every line beneath it and the diff then claims the whole literal changed, a cost only the author can weigh.

Two shapes draw no suggestion, and they draw different findings. A literal with no interior whitespace has nowhere legal to break at all, so a URL, a hash, or a dense regex keeps the report ending *"with no legal reshape"*, which is what holds that phrase to meaning what it says. A literal that would fit whole one indent below its line needs no break either, since the overflow came from the width ahead of it, and wrapping it in parentheses unsplit is a reshape this rule does not offer, so the report stays bare rather than claiming nothing could be done.

<template #configuration>

<RuleConfigTable />

`suggest-string-splits` gates the suggested form alone rather than the report. With it off, an over-budget line whose literal could carry a break is still named, and it is named without the *"with no legal reshape"* ending, because one exists there whether or not the finding spells it out.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[line-overflow]` directive. The [**Lengths**](/reference/configuration#lengths) section of the configuration reference states the caps-are-hard-constraints contract this rule closes.

</template>

</RuleLayout>

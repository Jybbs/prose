---
caption : "Names any line still over its governing cap once no reshape can shorten it."
related : [call-layout, signature-layout, import-layout]
layout  : doc
---

# line-overflow

<RuleLayout rule="line_overflow">

The `*-line-length` caps are hard constraints, and every shaping rule resolves within them, so a call, collection, signature, or import that crosses its cap reshapes to one entry per line, and an alignment run reshapes an over-budget member before aligning. When no legal form satisfies the cap (*a deep indent, a long identifier, a single-name import already at its narrowest, a cap set below what the statement needs*), the narrowest form stands and `line-overflow` names it, so an unsatisfiable cap reads as a finding in `prose check` and as a squiggle in the sandbox rather than as a knob that did nothing.

A line inside an import statement answers to `import-line-length`, every other line to `code-line-length`. A line a layout rule could still split (*an inline call carrying arguments, a multi-element collection, a multi-name `from` import, a signature carrying parameters, a single-statement match arm, a leading docstring*) is left for that rule, so `line-overflow` surfaces only the remainder no split can shorten. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[line-overflow]` directive. The [**Lengths**](/reference/configuration#lengths) section of the configuration reference states the caps-are-hard-constraints contract this rule closes.

</template>

</RuleLayout>

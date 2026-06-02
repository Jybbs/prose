---
category : lint
family   : lint
caption  : "surfaces a single-use bare `import x` the author chose not to alias."
related  : [alphabetize, align-imports, blank-lines]
layout   : doc
---

# bare-imports

<RuleLayout rule="bare_imports">

A bare `import requests` followed by a lone `requests.get(...)` four pages later forces the reader to walk back up to the import block to recover what `requests` is. A package whose namespace is read across the module earns its bare form, because the name then carries genuine information at every call site, and an aliased import (*`import numpy as np`*) is the author's deliberate namespace handle. `bare-imports` leans on those two signals, flagging an unaliased bare import only when its namespace is read at most once, recommending the explicit `from package import name` rewrite and leaving the rewrite itself to a future migration pass that picks up the lint output.

The rule walks every `import` statement in the module, including nested ones inside function bodies, conditional blocks, and class bodies, gauging how broadly each namespace is read at module scope. An entry on the `allow` list preserves the bare form, including its dotted submodules (*`numpy.linalg` inherits the exemption from `numpy`*), and `allow-aliased` (*on by default*) exempts every aliased import. When a downstream migration pass acts on the lint output, the rewrite hands off cleanly to the rest of the import surface: [[alphabetize]] sorts the resulting block, [[align-imports]] aligns the `import` keyword, and [[blank-lines]] lands the gap between groups. The lint itself is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow` | list of module names | `[]` | Modules whose bare-import form is preserved whatever their read count |
| `allow-aliased` | bool | `true` | Exempt every aliased bare import (*`import x as y`*) from the rule |

The `allow` list holds bare package names, where any dotted submodule of an allowlisted package inherits the exemption. Set `allow-aliased` to `false` for a project that wants every import to name its symbols, aliased or not.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[bare-imports]` directive.

</template>

</RuleLayout>

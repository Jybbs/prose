---
caption : "Surfaces a narrowly-used bare import that `from x import …` would replace."
related : [alphabetize, align-imports, blank-lines]
layout  : doc
---

# bare-imports

<RuleLayout rule="bare_imports">

A bare `import os` whose only use is `os.environ`, however many times it appears, names a single symbol the reader could pull in directly with `from os import environ`. A namespace reached through many distinct attributes earns its bare form, because the prefix then organizes a wide surface a `from` import would only scatter, and an aliased import (*`import numpy as np`*) is the author's deliberate namespace handle. `bare-imports` leans on those signals, flagging an unaliased bare import only when its namespace is reached through at most `max-attributes` distinct attributes (*default 4*) and never as the bare object itself, recommending the explicit `from package import name` rewrite and leaving the rewrite itself to a future migration pass that picks up the lint output.

The rule weighs each imported namespace by the distinct attributes read off it at module scope, attribute reads nested inside functions and class bodies still resolving to the module-level binding. A namespace used as the bare object (*passed to a call, bound to another name*) cannot collapse into a `from` import, so it passes whatever its attribute count, and a function-local import sits outside the module scope the rule measures. An entry on the `allow` list preserves the bare form, including its dotted submodules (*`numpy.linalg` inherits the exemption from `numpy`*), and `allow-aliased` (*on by default*) exempts every aliased import. When a downstream migration pass acts on the lint output, the rewrite hands off cleanly to the rest of the import surface: [[alphabetize]] sorts the resulting block, [[align-imports]] aligns the `import` keyword, and [[blank-lines]] lands the gap between groups. The lint itself is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow` | list of module names | `[]` | Modules whose bare-import form is preserved whatever their attribute count |
| `allow-aliased` | bool | `true` | Exempt every aliased bare import (*`import x as y`*) from the rule |
| `max-attributes` | integer | `4` | Distinct-attribute count at or below which a bare import is flagged |

The `allow` list holds bare package names, where any dotted submodule of an allowlisted package inherits the exemption. Set `allow-aliased` to `false` for a project that wants every import to name its symbols, aliased or not. Lower `max-attributes` to flag only the narrowest imports, or raise it to catch wider ones.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[bare-imports]` directive.

</template>

</RuleLayout>

---
category : lint
family   : lint
caption  : "surfaces bare `from module import *` patterns sitting outside the configured allowlist."
related  : [alphabetize, align-imports, blank-lines]
layout   : doc
---

# bare-import-allowlist

<RuleLayout rule="bare_import_allowlist">

A handful of packages encourage the namespace-as-import style, where `pandas.DataFrame` and `numpy.linalg.norm` read clearly at every call site because the package name carries genuine information. Most packages don't, and a bare `import requests` followed by `requests.get(...)` four pages later forces the reader to walk back up to the import block. `bare-import-allowlist` surfaces every off-list bare import as a lint diagnostic, recommending the explicit `from package import name` rewrite, leaving the rewrite itself to a future migration pass that picks up the lint output.

The rule walks every `import` statement in the module, including nested ones inside function bodies, conditional blocks, and class bodies. An entry on the `allow` list preserves the bare form, including its dotted submodules (*`numpy.linalg` inherits the exemption from `numpy`*). When a downstream migration pass acts on the lint output, the rewrite hands off cleanly to the rest of the import surface: [[alphabetize]] sorts the resulting block, [[align-imports]] aligns the `import` keyword, and [[blank-lines]] lands the gap between groups. The lint itself is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow` | list of module names | `["numpy", "pandas"]` | Modules whose bare-import form is preserved |

The `allow` list holds bare package names, where any dotted submodule of an allowlisted package inherits the exemption.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[bare-import-allowlist]` directive.

</template>

</RuleLayout>

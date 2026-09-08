---
caption : "Reports an unaliased bare import reached through at most `max-attributes` distinct attributes, which a `from x import …` would replace."
related : [alphabetize-siblings, align-imports, space-statements]
layout  : doc
---

# bare-imports

<RuleLayout rule="bare_imports">

`bare-imports` reports a bare `import os` that a `from os import environ` would serve better, because the module reads only a few names off the namespace and never uses the module object itself. The rule fires on an unaliased import reached through at least one and at most `max-attributes` distinct attributes (*default <ConfigDefault rule="bare-imports" facet="max-attributes" />*), however many times each attribute repeats, and a `from` import then names each symbol in use directly. The finding recommends the explicit `from package import name` rewrite and leaves the rewrite itself to a later migration pass that reads the lint output. A namespace reached through many distinct attributes keeps its bare form, because the prefix then organizes a wide set of names a `from` import would scatter, and an aliased import (*`import numpy as np`*) is the author's chosen namespace handle, exempt while `exempt-aliased` stays on.

The rule counts the distinct attributes read off each imported namespace at module scope, and an attribute read inside a function or class body still resolves to the module-level binding and counts. A namespace used as the bare object (*passed to a call, bound to another name*) cannot collapse into a `from` import, so it passes whatever its attribute count, and an import inside a function sits outside the module scope the rule measures. An entry on the `allow` list keeps its bare form, and an aliased import is exempt while `exempt-aliased` stays on. When a migration pass acts on the lint output, the other import rules finish the job, in that [[alphabetize-siblings]] sorts the resulting block, [[align-imports]] pads the `import` keyword to one column, and [[space-statements]] sets the blank lines between groups. The lint never rewrites, so the diagnostic is reported and the source stays as written.


<template #facets>

### `allow`

`allow` names the packages to leave alone, so `allow = ["numpy"]` keeps `import numpy` bare however many attributes the module reads off it. A dotted submodule inherits its parent's entry, so a `numpy` entry covers `numpy.linalg` too.

### `exempt-aliased`

`exempt-aliased` keeps an aliased import out of the report, since the `np` in `import numpy as np` is the namespace name the author chose. Setting it to `false` measures an aliased import on the same attribute count as a bare one, which suits a project where every import is to name its symbols.

### `max-attributes`

`max-attributes` sets the widest namespace the rule still reports, defaulting to <ConfigDefault rule="bare-imports" facet="max-attributes" />, so an import reached through more distinct attributes than that keeps its bare form. Lowering it leaves only the narrowest imports reported, and raising it reaches wider ones.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[bare-imports]` directive.

</template>

</RuleLayout>

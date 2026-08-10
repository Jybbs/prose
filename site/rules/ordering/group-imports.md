---
caption : "Partitions a module's imports into `__future__`, bare, external `from`, and local-package sections."
related : [alphabetize-siblings, align-imports, reflow-imports, space-statements, bare-imports]
layout  : doc
---

# group-imports

<RuleLayout rule="group_imports">

`group-imports` partitions a contiguous import run into its canonical sections, a `from __future__` import ahead of everything, then the bare `import` statements, the external `from … import …` statements, and the local-package imports last:

| Section | Holds |
|---|---|
| **`__future__`** | `from __future__ import annotations` |
| **Bare** | `import os`, `import numpy as np` |
| **External `from`** | `from collections import Counter` |
| **Local-package** | relative imports and any package on the `first-party` list |

The rule relocates imports into their section, leaving the order within each to [[alphabetize-siblings]], so the two agree on the grouping through one shared classifier. A run already sitting in section order passes through with no edit.

An absolute `from __future__ import …` takes the leading section on its own, because Python rejects a module that places the statement below any other code, so the section is a compiler requirement rather than a legibility preference. A relative `from .__future__ import …` and a bare `import __future__` name ordinary modules and classify as any other import would.

A `from` import is local when it is relative (*`from . import x`, `from ..pkg import y`*) or its module's root package appears on the `first-party` list. A bare `import` is local when any aliased root package is first-party. Everything else outside the standard `from` shape stays bare, and a name the rule cannot classify is no import at all and pins where it sits, ending the run.

A recognized **section marker** *(a hand-drawn banner like `# --- Typing ---` or a `##` hash heading)* divides a run into independent sections, so an author who grouped imports under a divider keeps that grouping and no import crosses the marker into the section above it. [[space-statements]] owns the single blank line dividing one canonical section from the next, [[reflow-imports]] runs first so every module reaches the partition on its own line, and [[align-imports]] acts on the grouped result, aligning the `import` keyword within each section.

Pair with [[alphabetize-siblings]] to sort the names within each section, with [[align-imports]] to align the `import` keyword across the freshly grouped block, and with [[space-statements]] for the blank line between sections.

<template #configuration>

<RuleConfigTable />

`group-imports` is a single on/off toggle. Left on, it partitions every import run into the canonical sections. Turned off with `group-imports = false`, the imports read as one flat block and [[alphabetize-siblings]] sorts them together rather than within sections. The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* names the packages that join the local-package section alongside relative imports.

</template>

</RuleLayout>

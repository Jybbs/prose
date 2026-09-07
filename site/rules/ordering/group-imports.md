---
caption : "Moves each import in a run into its section, `__future__` first, then bare, then external `from`, then local-package."
related : [alphabetize-siblings, align-imports, reflow-imports, space-statements, bare-imports]
layout  : doc
---

# group-imports

<RuleLayout rule="group_imports">

`group-imports` moves each import in a contiguous run into its canonical section, a `from __future__` import ahead of everything, then the bare `import` statements, the external `from … import …` statements, and the local-package imports last:

| Section | Members |
|---|---|
| **`__future__`** | `from __future__ import annotations` |
| **Bare** | `import os`, `import numpy as np` |
| **External `from`** | `from collections import Counter` |
| **Local-package** | relative imports and any package on the `first-party` list |

The rule moves imports into their sections and leaves the order within each to [[alphabetize-siblings]], so the two agree on the grouping through one shared classifier. A run already in section order passes through with no edit.

An absolute `from __future__ import …` takes the leading section on its own, because Python rejects a module that places the statement below any other code, so the section is a compiler requirement rather than a legibility preference. A relative `from .__future__ import …` and a bare `import __future__` name ordinary modules and classify as any other import does.

A `from` import is local when it is relative (*`from . import x`, `from ..pkg import y`*) or its module's root package appears on the `first-party` list. A bare `import` is local when the root package of any name it binds is first-party. Every other bare `import` stays bare, every other `from` import is external, and a statement that is no import at all stays where it sits and ends the run.

A recognized **section marker** *(a hand-drawn banner like `# --- Typing ---` or a `##` hash heading)* divides a run into independent sections, so an author who grouped imports under a divider keeps that grouping and no import crosses the marker into the section above it. [[space-statements]] owns the single blank line between one canonical section and the next, [[reflow-imports]] runs afterward and splits a comma-joined statement so each module sits on its own line in its section, and [[align-imports]] reads the grouped result and aligns the `import` keyword within each section.

<template #configuration>

<RuleConfigTable />

`group-imports` is a single on/off toggle, and left on it moves every import run into the canonical sections. Turned off with `group-imports = false`, the imports read as one flat block and [[alphabetize-siblings]] sorts them together rather than within sections. The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* names the packages that join the local-package section alongside relative imports.

</template>

</RuleLayout>

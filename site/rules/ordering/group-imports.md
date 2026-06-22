---
caption : "Partitions a module's imports into bare, external `from`, and local-package sections."
related : [alphabetize, align-imports, import-layout, blank-lines, bare-imports]
layout  : doc
---

# group-imports

<RuleLayout rule="group_imports">

A reader scanning a module's head wants to know at a glance what it draws on and from where. When the imports arrive in the order they were typed, the standard library, the third-party packages, and the project's own modules tangle together, and the reader has to read each line to place it. `group-imports` **partitions a contiguous import run into three canonical sections**, the bare `import` statements first, the external `from … import …` statements next, and the local-package imports last:

| Section | Holds |
|---|---|
| **Bare** | `import os`, `import numpy as np` |
| **External `from`** | `from collections import Counter` |
| **Local-package** | relative imports and any package on the `first-party` list |

The rule **relocates** imports into their section, the move that makes the grouping a structural concern rather than an alphabetizing one. It leaves the order of the names within a section untouched, the sort within each left to [[alphabetize]], so the two agree on the grouping through one shared classifier rather than each deciding membership on its own. A run already sitting in section order passes through with no edit.

A `from` import is local when it is relative (*`from . import x`, `from ..pkg import y`*) or its module's root package appears on the `first-party` list. A bare `import` is local when any aliased root package is first-party. Everything else outside the standard `from` shape stays bare, and a name the rule cannot classify is no import at all and pins where it sits, ending the run.

A recognized **section marker** *(a hand-drawn banner like `# --- Typing ---` or a `##` hash heading)* divides a run into independent sections, so an author who grouped imports under a divider keeps that grouping and no import crosses the marker into the section above it. [[blank-lines]] owns the single blank line dividing one canonical section from the next, and [[align-imports]] and [[import-layout]] act on the grouped result, aligning the `import` keyword within each section and splitting an over-long `from` import across lines.

Pair with [[alphabetize]] to sort the names within each section, with [[align-imports]] to align the `import` keyword across the freshly grouped block, and with [[blank-lines]] for the blank line between sections.

<template #configuration>

<RuleConfigTable />

`group-imports` is a single on/off toggle. Left on, it partitions every import run into the canonical sections. Turned off with `group-imports = false`, the imports read as one flat block and [[alphabetize]] sorts them together rather than within sections. The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* names the packages that join the local-package section alongside relative imports.

</template>

</RuleLayout>

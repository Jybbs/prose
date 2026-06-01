---
category : auto-fix
family   : ordering
caption  : "alphabetizes import siblings, dict-key blocks, and dataclass field runs."
related  : [align-colons, align-imports, bare-imports, blank-lines]
layout   : doc
---

# alphabetize

<RuleLayout rule="alphabetize">

A reader who already knows the codebase carries a **mental map** of where things live. When sibling members within a class, an enum, a dataclass, or a function call sit in arrival order, every reader builds a **different map**, which slows each new reader's first read. `alphabetize` gives everyone the **same landmarks**:

| Surface | Order |
|---|---|
| **Classes in a module** | Alphabetical |
| **Module-level assignments** | Within each dependency tier |
| **Methods in a class** | Dunders, properties, private, public |
| **Enum members** | Alphabetical |
| **Dataclass and Pydantic fields** | Required before optional |
| **Parameters and keyword arguments** | Alphabetical |
| **Imports** | Canonical groups, alphabetical within each |
| **Docstring entries** | Alphabetical within each Title-case section |

The rule fires on siblings whose order does not carry meaning. It leaves alone every surface where ordering is load-bearing (*positional-only parameters before the `/` separator, enum members with explicit integer or string values, tuple-unpacking targets bound to positional results*).

When a function's parameters reorder, `alphabetize` rewrites each in-module call's positional arguments to keyword form, keyed to the parameter each bound to under the original order, so the reorder never silently rebinds a caller. Calls that forward `*args`, unpack `**`, or reach the function through a reassigned name stay as they read.

Pair with [[align-imports]] to align the `import` keyword across the freshly-sorted block, with [[align-colons]] to align dataclass-field annotations after the sort, and with [[blank-lines]] for the blank-line discipline around class members and the single blank line between the import groups.

<template #configuration>

<RuleConfigTable />

The ordering itself follows fixed per-construct conventions. Method groups follow the dunders-properties-privates-publics rhythm. Pydantic fields follow required-then-optional. Consecutive imports group into their canonical order (*bare first, then external `from`, then local-package*), sorted within each group, with the `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* naming the packages that lift into the local-package group alongside relative imports. Set `alphabetize = { docstring-entries = false }` to skip the docstring-entry reorder while keeping every AST-level surface sorted.

</template>

</RuleLayout>

---
caption : "Alphabetizes import siblings, dict-key blocks, and dataclass field runs."
related : [align-colons, align-imports, band-constants, bare-imports, blank-lines, group-imports, unsorted-parameters]
layout  : doc
---

# alphabetize

<RuleLayout rule="alphabetize">

A reader who already knows the codebase carries a **mental map** of where things live. When sibling members within a class, an enum, a dataclass, or a function call sit in arrival order, every reader builds a **different map**, which slows each new reader's first read. `alphabetize` gives everyone the **same landmarks**:

| Surface | Order |
|---|---|
| **Classes in a module** | Alphabetical |
| **Methods in a class** | Dunders, properties, private, public |
| **Enum members** | Alphabetical |
| **Dataclass and Pydantic fields** | Required before optional |
| **Parameters and keyword arguments** | Keyword-only and call keywords alphabetical, positional held |
| **Imports** | Alphabetical within each [[group-imports]] section |
| **Docstring entries** | Parameter entries mirror the signature, all else alphabetical |

The rule fires on siblings whose order does not carry meaning. It leaves alone every surface where ordering is load-bearing (*positional-only parameters before the `/` separator, enum members with explicit integer or string values, tuple-unpacking targets bound to positional results*).

A class or function definition also holds its place behind any sibling it names at evaluation time (*a base class, a decorator, a parameter default, a non-deferred annotation, or a class-body value*), since sorting it ahead of that sibling would move the name out of scope and raise `NameError` on import. A run whose references form a cycle stays in source order untouched.

Inside a class body, the constants *(bare assignments and `ClassVar`-annotated values)* and the annotated data fields tier through one shared graph, so a constant a method default, base class, or decorator reads stays above the definition reading it, and each `ClassVar` value sorts among the constants while every other annotation holds its place on the required-before-optional field run. Where honoring a reference would strand a reader, the constrained members keep source order.

A recognized **section marker** splits a sort run into sections that each alphabetize on their own while the marker holds its place, so an author who groups members under a divider keeps that grouping through the sort. A hand-drawn banner *(an own-line comment whose body is a run of repeated rule characters around an optional label, `# --- Lifecycle ---`)* and a `##` hash heading both read as dividers, across a class body, a module, and an import group alike. An ordinary prose comment is not a divider, so it stays attached to the member directly below it and travels with that member through the sort.

`alphabetize` never reorders a function's positional-or-keyword parameters, free function and method alike, because a parameter's slot is part of the call contract and a single-file formatter cannot reach the callers a reorder would rebind (*another module's positional call, a framework invoking a hook by slot, a dynamically-dispatched method*). The keyword-only block past the `*` still sorts, since a keyword-only parameter binds by name at every call site. The companion [[unsorted-parameters]] lint reports a free function whose positional run sits out of order, where a reorder is at least worth a human's read of the callers.

At a call site, keyword arguments already in `name=value` form alphabetize, on any callee including a method, because their order never affects which parameter each binds. Positional arguments hold their slot, since naming them would require resolving the callee's signature, which *Prose* does only for a plain in-module function and never for a method.

A docstring entry naming a parameter of the signature it documents takes that parameter's position as the rule leaves the signature, which for the positional run is source order and for the keyword-only block is sorted order. An entry naming nothing in the signature (*a parameter renamed or removed since the docs were written*) sinks below the mirrored entries, stragglers alphabetizing among themselves. A section with no parameter entries (*`Raises:`, `Returns:`*) alphabetizes throughout.

Pair with [[align-imports]] to align the `import` keyword across the freshly-sorted block, with [[align-colons]] to align dataclass-field annotations after the sort, and with [[blank-lines]] for the blank-line discipline around class members and the single blank line between the import groups.

<template #configuration>

<RuleConfigTable />

The ordering itself follows fixed per-construct conventions. Method groups follow the dunders-properties-privates-publics rhythm. Pydantic fields follow required-then-optional. [[group-imports]] partitions consecutive imports into their canonical sections (*bare first, then external `from`, then local-package*) and `alphabetize` sorts the names within each, the `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* naming the packages that lift into the local-package section alongside relative imports. Set `alphabetize = { sort-docstring-entries = false }` to skip the docstring-entry reorder while keeping every AST-level surface sorted.

</template>

</RuleLayout>

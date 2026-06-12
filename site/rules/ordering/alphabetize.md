---
caption : "alphabetizes import siblings, dict-key blocks, and dataclass field runs."
related : [align-colons, align-imports, bare-imports, blank-lines]
layout  : doc
---

# alphabetize

<RuleLayout rule="alphabetize">

A reader who already knows the codebase carries a **mental map** of where things live. When sibling members within a class, an enum, a dataclass, or a function call sit in arrival order, every reader builds a **different map**, which slows each new reader's first read. `alphabetize` gives everyone the **same landmarks**:

| Surface | Order |
|---|---|
| **Classes in a module** | Alphabetical |
| **Module-level constants** | Leading and trailing bands, each by dependency tier |
| **Methods in a class** | Dunders, properties, private, public |
| **Enum members** | Alphabetical |
| **Dataclass and Pydantic fields** | Required before optional |
| **Parameters and keyword arguments** | Alphabetical, method positionals pinned |
| **Imports** | Canonical groups, alphabetical within each |
| **Docstring entries** | Alphabetical within each Title-case section |

The rule fires on siblings whose order does not carry meaning. It leaves alone every surface where ordering is load-bearing (*positional-only parameters before the `/` separator, enum members with explicit integer or string values, tuple-unpacking targets bound to positional results*).

A class or function definition also holds its place behind any sibling it names at evaluation time (*a base class, a decorator, a parameter default, a non-deferred annotation, or a class-body value*), since sorting it ahead of that sibling would move the name out of scope and raise `NameError` on import. A run whose references form a cycle stays in source order untouched.

At module scope, a constant lifts out of arrival order into a band. One whose value reaches only imports, builtins, literals, or fellow leading constants **rides a leading band directly below the imports**, and one that names a function or class **pools into a trailing band beneath the definitions**, so a module reads as its imports, its leading constants, its definitions, then the constants derived from them. Each band alphabetizes within its dependency tier, and a constant a function reads only inside its body still hoists above it, because only an eval-time reference *(a right-hand side, a decorator, a default argument, a base class, a non-deferred annotation)* binds the order. A constant the rule cannot place safely (*a reassigned name, an unresolved reference, a line a suppression directive or a `# prose: keep` marker covers*) pins where the author left it.

When a function's parameters reorder, `alphabetize` rewrites each in-module call's positional arguments to keyword form, keyed to the parameter each bound to under the original order, so the reorder never silently rebinds a caller. Calls that forward `*args`, unpack `**`, or reach the function through a reassigned name stay as they read.

A function defined in a class body never reorders its positional-or-keyword parameters, whatever its decorators or in-module call sites, because a method's callers routinely live outside the module (*a framework invoking an overridden hook positionally, external code calling through the class's contract*) where no call-site rewrite can reach. A docstring section whose every entry names a parameter of such a pinned signature holds the same order, mirroring the signature it documents. Keyword-only parameters keep sorting wherever they appear, since they bind by name at every call site.

Pair with [[align-imports]] to align the `import` keyword across the freshly-sorted block, with [[align-colons]] to align dataclass-field annotations after the sort, and with [[blank-lines]] for the blank-line discipline around class members and the single blank line between the import groups.

<template #configuration>

<RuleConfigTable />

The ordering itself follows fixed per-construct conventions. Method groups follow the dunders-properties-privates-publics rhythm. Pydantic fields follow required-then-optional. Consecutive imports group into their canonical order (*bare first, then external `from`, then local-package*), sorted within each group, with the `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* naming the packages that lift into the local-package group alongside relative imports. Set `alphabetize = { docstring-entries = false }` to skip the docstring-entry reorder while keeping every AST-level surface sorted.

</template>

</RuleLayout>

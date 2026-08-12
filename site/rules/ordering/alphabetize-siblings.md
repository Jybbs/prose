---
caption : "Alphabetizes import siblings, dict-key blocks, and class-body members."
related : [align-colons, align-imports, band-constants, bare-imports, space-statements, group-imports, unsorted-positionals]
layout  : doc
---

# alphabetize-siblings

<RuleLayout rule="alphabetize_siblings">

`alphabetize-siblings` sorts sibling members whose order carries no meaning, so every reader meets the same landmarks:

| Surface | Order |
|---|---|
| Classes in a module | Alphabetical |
| Methods in a class | Dunders, properties, private, public |
| Enum members | Alphabetical |
| Pydantic `BaseModel` and `TypedDict` fields | Required before optional |
| Dataclass and `NamedTuple` fields | Source order held |
| Parameters and keyword arguments | Keyword-only and call keywords alphabetical, positional held |
| Dict literal keys | Scalar entries before collection entries, alphabetical within each |
| Imports | Alphabetical within each [[group-imports]] section |
| Docstring entries | Parameter entries mirror the signature, all else alphabetical |

Order that is load-bearing stays untouched, covering positional-only parameters ahead of the `/`, enum members carrying explicit values, and tuple-unpacking targets.

A definition holds its place behind any sibling it names at evaluation time (*a base class, a decorator, a parameter default, a non-deferred annotation, a class-body value*), and a module-level statement reading one binds its run the same way. A decorated definition at module scope holds its slot outright whereas a decorated method still sorts, and a reference cycle leaves its run in source order. Inside a class body the constants and the annotated fields tier through one graph, so a constant a method default or base class reads stays above it.

A section marker splits a run into sections that each sort on their own while it holds its place, covering a banner (*`# --- Lifecycle ---`*), the same banner drawn with its rule closing the label rather than opening it (*`# Lifecycle -------#`*), a `##` heading, and a suppression directive. An ordinary comment is no divider and travels with the member below it.

Positional-or-keyword parameters never reorder, a slot being part of the call contract, whereas the keyword-only block past the `*` sorts and [[unsorted-positionals]] reports a run out of order. A class whose header generates its constructor answers to the same contract, so a `NamedTuple` or `msgspec.Struct` base and a `@dataclass`, `attrs`, or `pydantic.dataclasses` decorator each pin the field run, whereas `kw_only=True`, a `dataclasses.KW_ONLY` block, a `TypedDict`, and a `pydantic.BaseModel` sort throughout.

At a call site, keyword arguments in `name=value` form sort on any callee while positional arguments hold their slots. Dict keys sort by default, and since insertion order is observable through iteration, `.items()`, and `**` expansion, `sort-dict-keys = false` holds every dict in a project and `# prose: keep` holds one literal. In both a call and a dict, an entry whose value runs code (*a call, a comprehension, an `await`*) holds its slot, and set literals sort regardless.

A docstring entry naming a parameter takes that parameter's position as the rule leaves the signature and an entry naming nothing there sinks below the mirrored ones.

<template #configuration>

<RuleConfigTable />

The ordering itself follows fixed per-construct conventions. Method groups follow the dunders-properties-privates-publics rhythm. Pydantic fields follow required-then-optional. [[group-imports]] partitions consecutive imports into their canonical sections (*a `from __future__` import first, then bare, then external `from`, then local-package*) and `alphabetize-siblings` sorts the names within each, the `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* naming the packages that lift into the local-package section alongside relative imports. Each sort pass also switches off on its own through the facets above, so a project can keep its methods grouped while leaving its definitions in source order, or hold a hand-curated `__all__` while every other surface still sorts.

</template>

</RuleLayout>

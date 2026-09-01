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
| Classes and functions in a module | Classes above functions, alphabetical within each band |
| Methods in a class | Dunders, properties, private, public |
| Enum members | Alphabetical |
| Pydantic `BaseModel` and `TypedDict` fields | Required before optional |
| Dataclass and `NamedTuple` fields | Source order held |
| Parameters and keyword arguments | Keyword-only and call keywords alphabetical, positional held |
| Dict literal keys | Scalar entries before collection entries, alphabetical within each |
| Imports | Alphabetical within each [[group-imports]] section |
| Docstring entries | Parameter entries mirror the signature, all else alphabetical |

Order that is load-bearing stays untouched, covering positional-only parameters ahead of the `/`, enum members carrying explicit values, and tuple-unpacking targets.

At module scope the classes and the functions sort as one run with every class seated above every function, each band alphabetical and the function band taking the dunder, private, public grouping the methods take, so a module reads its classes first and its functions below them whatever order the author interleaved them in.

A definition holds its place behind any sibling it names at evaluation time (*a base class, a decorator, a parameter default, a non-deferred annotation, a class-body value*), and a module-level statement reading one binds its run the same way. A module-level statement binding a name pins the run too, covering an assignment, an unpack target, a `for` or `with` target, a walrus, each alias of an import, an `except ... as` name, and a `del`, so a definition naming it keeps the side of that binding the source seated it on, a reader below never rising above and one above never sinking below. A module-level call reaches through to what it runs, binding the run against the names its target reads at evaluation time, a method call on a class reaching the whole class body, and a definition reaches the same way through the decorators, bases, metaclass, and defaults it evaluates as it binds. A class whose base list runs a call or a subscript on a name the module does not itself define fences the run at its own slot, because a metaclass and a `__class_getitem__` hook both run at class creation and a hook reached through a compiled module calls back into the module that imported it, so no static read follows where it reaches. The fence is one-sided, in that nothing written above such a class may sort below it whereas every definition written below was unbound when the hook ran and still sorts freely. An enumeration whose members take their value from `auto` or from a `__new__` numbering them as it runs holds the order its author wrote, since sorting it would rewrite what each member holds, whereas one spelling every value out still sorts. A decorated definition at module scope holds its slot outright whereas a decorated method still sorts, and a reference cycle leaves its run in source order. Inside a class body the constants and the annotated fields tier through one graph, so a constant a method default or base class reads stays above it.

A section marker splits a run into sections that each sort on their own while it holds its place, covering a banner (*`# --- Lifecycle ---`*), the same banner drawn with its rule closing the label rather than opening it (*`# Lifecycle -------#`*), a `##` heading, and a suppression directive. An ordinary comment is no divider and travels with the member below it, and a group packing several members onto a row holds its order across one, since its members swap in place with every gap kept verbatim and the comment would stay put while they flowed past it.

Positional-or-keyword parameters never reorder, a slot being part of the call contract, whereas the keyword-only block past the `*` sorts and [[unsorted-positionals]] reports a run out of order. A class whose header generates its constructor answers to the same contract, so a `NamedTuple` or `msgspec.Struct` base and a `@dataclass`, `attrs`, or `pydantic.dataclasses` decorator each pin the field run, whereas `kw_only=True`, a `dataclasses.KW_ONLY` block, a `TypedDict`, and a `pydantic.BaseModel` sort throughout.

At a call site, keyword arguments in `name=value` form sort on any callee while positional arguments hold their slots. Dict keys sort by default, and since insertion order is observable through iteration, `.items()`, and `**` expansion, `sort-dict-keys = false` holds every dict in a project and `# prose: keep` holds one literal, the same marker holding one `__all__` or `__slots__` where `sort-dunder-lists = false` holds them all. In both a call and a dict, an entry whose value runs code (*a call, a comprehension, an `await`*) holds its slot, and set literals sort regardless.

A docstring entry naming a parameter takes that parameter's position as the rule leaves the signature and an entry naming nothing there sinks below the mirrored ones.

<template #configuration>

<RuleConfigTable />

The ordering itself follows fixed per-construct conventions. Method groups follow the dunders-properties-privates-publics rhythm. Pydantic fields follow required-then-optional. [[group-imports]] partitions consecutive imports into their canonical sections (*a `from __future__` import first, then bare, then external `from`, then local-package*) and `alphabetize-siblings` sorts the names within each, the `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* naming the packages that lift into the local-package section alongside relative imports. Each sort pass also switches off on its own through the facets above, so a project can keep its methods grouped while leaving its definitions in source order, or hold a hand-curated `__all__` while every other surface still sorts.

</template>

</RuleLayout>

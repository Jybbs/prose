---
caption : "Sorts sibling entries whose order carries no meaning, covering import names, dict keys, class-body members, keyword arguments, and docstring entries."
related : [align-colons, align-imports, band-constants, bare-imports, space-statements, group-imports, unsorted-positionals]
layout  : doc
---

# alphabetize-siblings

<RuleLayout rule="alphabetize_siblings">

`alphabetize-siblings` sorts sibling members whose order carries no meaning, so every reader meets the same landmarks:

| Construct | Order |
|---|---|
| Classes and functions in a module | Classes above functions, alphabetical within each band |
| Methods in a class | Dunders, properties, private, public |
| Enum members | Alphabetical |
| Pydantic `BaseModel` and `TypedDict` fields | Required before optional |
| Dataclass and `NamedTuple` fields | Source order kept |
| Parameters and keyword arguments | Keyword-only parameters and call keywords alphabetical, positional kept in place |
| Dict literal keys | Scalar entries before collection entries, alphabetical within each |
| Imports | Alphabetical within each [[group-imports]] section |
| Docstring entries | Parameter entries follow the signature order, all else alphabetical |

Order that carries meaning stays as written, covering positional-only parameters ahead of the `/`, enum members whose values come from `auto` or from a `__new__` that numbers them, and tuple-unpacking targets.

At module scope the classes and the functions sort as one run with every class above every function, each band alphabetical and the function band grouped as dunder, private, public the way methods are, so a module reads its classes first and its functions below them whatever order the author interleaved them in.

A definition stays behind any sibling it names at evaluation time (*a base class, a decorator, a parameter default, a non-deferred annotation, a class-body value*), and a module-level statement that reads one pins its run the same way. Several other statements pin a run or fence it:

- A module-level statement that binds a name pins the run, covering an assignment, an unpack target, a `for` or `with` target, a walrus, each alias of an import, an `except ... as` name, and a `del`. A definition naming that name then keeps the side of the binding the source put it on, where a reader below never rises above it and one above never sinks below it.
- A module-level call reaches through to what it runs, pinning the run against the names its target reads at evaluation time, where a method call on a class reaches the whole class body, and a definition reaches the same way through the decorators, bases, metaclass, and defaults it evaluates as it binds.
- A class whose base list runs a call or a subscript on a name the module does not itself define fences the run at its own slot. The fence holds because a metaclass and a `__class_getitem__` hook both run at class creation, and a hook reached through a compiled module calls back into the module that imported it, so no static read follows where it reaches. The fence is one-sided, in that nothing written above such a class may sort below it, whereas every definition written below it was unbound when the hook ran and still sorts freely.
- An enumeration whose members take their value from `auto` or from a `__new__` that numbers them as it runs keeps the order its author wrote, since sorting it would change each member's value, whereas one that spells every value out still sorts.
- A decorated definition at module scope keeps its slot outright whereas a decorated method still sorts, and a reference cycle leaves its run in source order.
- Inside a class body the constants and the annotated fields sort through one dependency graph, so a constant a method default or base class reads stays above it.

A section marker splits a run into sections that each sort on their own while the marker keeps its place, covering a banner (*`# --- Lifecycle ---`*), the same banner drawn with its rule closing the label rather than opening it (*`# Lifecycle -------#`*), a `##` heading, and a suppression directive. An ordinary comment is no divider and travels with the member below it, and a group that packs several members onto one row keeps its order across a comment, because its members swap in place with every gap kept as written and the comment would stay put while they moved past it.

Positional-or-keyword parameters never reorder, since a slot is part of the call contract, whereas the keyword-only block past the `*` sorts, and [[unsorted-positionals]] reports a run out of order. A class whose header generates its constructor follows the same contract, so a `NamedTuple` or `msgspec.Struct` base and a `@dataclass`, `attrs`, or `pydantic.dataclasses` decorator each pin the field run, whereas `kw_only=True`, a `dataclasses.KW_ONLY` block, a `TypedDict`, and a `pydantic.BaseModel` sort throughout.

At a call site, keyword arguments in `name=value` form sort on any callee while positional arguments keep their slots. Dict keys sort by default, and because insertion order is observable through iteration, `.items()`, and `**` expansion, `sort-dict-keys = false` keeps every dict in a project as written and `# prose: keep` keeps one literal. The same marker keeps one `__all__` or `__slots__` where `sort-dunder-lists = false` keeps them all. In both a call and a dict, an entry whose value runs code (*a call, a comprehension, an `await`*) keeps its slot, and set literals sort regardless.

A docstring entry naming a parameter takes that parameter's position as the rule leaves the signature, and an entry naming nothing in the signature sinks below the mirrored ones.

<template #configuration>

<RuleConfigTable />

The order itself follows fixed per-construct conventions, with method groups following the dunders, properties, privates, publics order and Pydantic fields following required then optional. [[group-imports]] moves consecutive imports into their canonical sections (*a `from __future__` import first, then bare, then external `from`, then local-package*) and `alphabetize-siblings` sorts the names within each, with the `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* naming the packages that join the local-package section alongside relative imports. Each sort also switches off on its own through the facets above, so a project can keep its methods grouped while leaving its definitions in source order, or keep a hand-curated `__all__` while everything else still sorts.

</template>

</RuleLayout>

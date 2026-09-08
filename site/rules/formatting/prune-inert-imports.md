---
caption : "Removes an import that binds a name nothing references, or that repeats a binding an earlier import already made, and reports rather than removes the unreferenced one where the file reads as a compatibility shim or a package `__init__.py`."
related : [bare-imports, group-imports, inlinable-bindings, modernize-annotations]
layout  : doc
---

# prune-inert-imports

<RuleLayout rule="prune_inert_imports">

`prune-inert-imports` removes an import that binds a name nothing references, under `drop-unreferenced`, and a second import that rebinds a name an earlier import already bound, under `drop-duplicates`. Both facets read the binding table [[inlinable-bindings]] reads, and where removing an import could change what another module imports from this one, `drop-unreferenced` reports the binding rather than removing it.

<Fixture rule="prune_inert_imports" case="repeat_and_unread_member_both_go" />

The reference count runs per bound name, so one member drops off a shared `from` import while its siblings stay.

<Fixture rule="prune_inert_imports" case="every_member_unread_drops_the_whole_line" />

A repeat matches on both the name it binds and the path it names, so `import os` beside `import os.path` is two imports rather than a repeat.

## What Holds Its Line

Whether a name is re-exported is a fact about *other* files, and *Prose* formats one file at a time, so it never sees the sibling doing `from shim import name`. Two shapes in the file itself say that removing an import could break such a sibling, and in both the rule reports the binding rather than removing it.

The first is a module that writes no `__all__` and binds no name of its own, no `def`, no `class`, and no assignment outside a `try` or a version branch. A file whose entire content is imports and the branches guarding them is not using those names, it is carrying them, which is what a compatibility shim is.

<Fixture rule="prune_inert_imports" case="module_binding_no_name_of_its_own_reports_instead" />

`drop-duplicates` is unaffected, because removing a repeat cannot change what the module re-exports while the first import still binds the name.

<Fixture rule="prune_inert_imports" case="repeat_drops_while_the_binding_it_repeats_reports" />

The second is a package `__init__.py`, whose bindings are the package's public API whatever its `__all__` says.

Everywhere else the removal stands. A module that writes `__all__` has stated its public surface, so an unreferenced import drops there even where that surface is empty.

<Fixture rule="prune_inert_imports" case="empty_dunder_all_drops_an_unread_name" />

A module that binds names of its own drops one too, whatever its `__all__`. That leaves one case no signal inside the file reaches, a shim carrying a single helper function beside its re-exports, which reads as an ordinary module and loses them. Writing one of the markers below is what settles it.

An import carrying a re-export marker holds its line under both facets, so a repeated self-alias survives `drop-duplicates`:

1. A name listed in `__all__`.
2. The PEP 484 redundant-alias form `from x import y as y`.
3. A `noqa` comment trailing the import, either bare or naming `F401`, which keeps every name that statement binds. The marker has to open a comment rather than appear inside its text, so a stacked `# type: ignore  # noqa: F401` counts whereas a sentence mentioning the word does not, and a statement spanning several rows carries it on the row it opens or the row it closes.
4. A name taken from a module whose own name marks it private (*`from _ssl import OPENSSL_VERSION`*), which is how a public module re-exports its implementation. A dunder module such as `__future__` is excluded, because its names carry compiler meaning rather than a public API.
5. A file-level pragma on its own line naming the unused-import behavior, being `# ruff: noqa: F401`, `# flake8: noqa: F401`, or `# pyright: reportUnusedImport=false`, which holds every unreferenced import in the module at once. Each is what the tool naming it already reads as *"the unused imports here are deliberate"*, so *Prose* reads it the same way rather than asking for a spelling of its own. A head naming no code (*a bare `# ruff: noqa`*) is not read, because it silences every rule its tool carries and so says nothing about re-exports in particular.

Two of those markers are written in a comment rather than in code, which is where the wider ecosystem records a re-export no static read can see. [[band-constants]] reads a `noqa` naming `E402` as pinning an import to the row its author gave it. Those readings are the whole set, so neither a `noqa` nor a file-level pragma exempts anything from any other rewrite or lint in *Prose*.

<Fixture rule="prune_inert_imports" case="self_alias_marks_a_reexport" />

An `__all__` built from anything other than a list or tuple of string literals, written below module scope, or changed after its assignment keeps every import in that module, as does a `from … import *`. A change means an `append`, an `extend`, or a write through a subscript such as `__all__[:] = sorted(__all__)`.

Two reads the reference count misses keep an import too. A `del` of the bound name needs that binding to exist, and a name read only inside a quoted type expression sits in a string literal rather than in the tree the table reads, so the rule parses each one for the names it reads. A quoted type sits in an annotation or in one of the typing constructs that takes a type written as a string, being `cast`, `assert_type`, `NamedTuple`, `NewType`, `TypeAliasType`, `TypeVar`, and `TypedDict`. Each of those but `cast` names the new type in its first argument, so the rule reads the type from the argument after it.

<Fixture rule="prune_inert_imports" case="quoted_type_holds_its_import" />

An import binding `__all__` itself sets the whole export surface, so it stays too, as does a name a second import rebinds from another source, which keeps the fallback in a `try: from _speedups import loads` shim in place.

A repeat of a name nothing reads takes the first binding with it, because both facets resolve in the one pass rather than one per run.

<Fixture rule="prune_inert_imports" case="repeat_of_an_unread_name_drops_both_lines" />

An own-line comment directly above an import keeps the whole statement, because removing the line would leave the comment above whatever statement follows. Where [[reflow-imports]] will merge the statement into a same-module sibling, the drop happens instead on the merged line the comment then leads.

<Fixture rule="prune_inert_imports" case="leading_comment_holds_its_import" />

## The `__future__` Directive

`from __future__ import annotations` is removed wherever the directive changes nothing at runtime:

1. `target-version` is 3.14 or higher, where PEP 749 defers evaluation.
2. No annotation runs at module scope, and every annotated name resolves to an unconditional module-scope binding written before it.

An annotation at module scope keeps the directive whatever its names resolve to, because the directive decides whether Python stores that annotation in the module's `__annotations__` as a string or evaluates it at import time, so removing it changes what the module presents. An annotation on a `def` or inside a `class` body is stored on that object instead, and the directive can be removed once every name the annotation reads is bound ahead of it.

Where [[alphabetize-siblings]] sorts definitions in the same pipeline, a name a module-level class or function binds counts as unresolved whichever side of the annotation it sits on, since the sort moves definitions after this rule has run. A directive covering such a reference therefore stays in whichever order the sort writes. Where [[band-constants]] runs in the same pipeline, a binding it hoists above the annotation naming it counts as written before that annotation, whether the hoist moves a constant into the leading band or an import into the import run, because the rule reads the module as the band places it once the directive is gone.

<Fixture rule="composition" case="hoisted_alias_settles_the_directive" />

Every other `__future__` feature stays, because `division` and its siblings change how the module compiles rather than binding a name.

<Fixture rule="prune_inert_imports" case="division_directive_out_of_scope" />

::: tabs key:prose-target-version
== Python 3.10
The version-gated branch does not run, so the directive goes only where the module carries no annotation or every annotation resolves against an earlier module-scope binding.

== Python 3.11
The branch does not run, the same as on 3.10.

== Python 3.12
The branch does not run, the same as on 3.10.

== Python 3.13
The branch does not run, the same as on 3.10.

== Python 3.14
The version-gated branch runs, because PEP 749 defers annotation evaluation and the directive changes nothing at runtime.
:::

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates the `__future__` branch per project.

</template>

<template #related-after>

For the gate semantics, [**`target-version`**](/reference/configuration#top-level-keys) in the Configuration chapter covers how the field is read across version-gated rules.

</template>

</RuleLayout>

---
caption : "Drops an import binding nothing references and a repeat of a binding an earlier import already made, reporting the one a package `__init__.py` re-exports."
lints   : true
related : [bare-imports, group-imports, modernize-annotations, single-use-variables]
layout  : doc
---

# prune-inert-imports

<RuleLayout rule="prune_inert_imports">

`prune-inert-imports` drops an import binding a name nothing references under `drop-unreferenced`, and a second import rebinding a name already bound under `drop-duplicates`, resolving both against the binding table [[single-use-variables]] reads.

<Fixture rule="prune_inert_imports" case="repeat_and_unread_member_both_go" />

The accounting runs per bound name, so a member prunes off a shared `from` import while its siblings stay.

<Fixture rule="prune_inert_imports" case="every_member_unread_drops_the_whole_line" />

A repeat matches on both the name it binds and the path it names, so `import os` beside `import os.path` is two imports.

## What Holds Its Line

Both facets read the same re-export markers, so a repeated self-alias survives `drop-duplicates`:

1. A name listed in `__all__`.
2. The PEP 484 redundant-alias form `from x import y as y`.
3. A `noqa` comment trailing the import, either bare or naming `F401`, which holds every name that statement binds. The marker opens a comment rather than sitting in its prose, so a stacked `# type: ignore  # noqa: F401` counts whereas a sentence mentioning the word does not, and a statement spanning several rows carries it on the row it opens or the row it closes.
4. A name taken from a module whose own name marks it private (*`from _ssl import OPENSSL_VERSION`*), which is how a public module re-exports its implementation. A dunder module such as `__future__` is excluded, its names carrying compiler meaning rather than a surface.

The `noqa` marker is the only one a reader writes in a comment rather than in code, and it is what the wider ecosystem puts on a re-export no static read can see. [[band-constants]] reads the same comment for `E402`, which pins an import to the row its author gave it. Those two readings are the whole of it, so a `noqa` opens nothing out of a rewrite or a lint anywhere else in *Prose*.

<Fixture rule="prune_inert_imports" case="self_alias_marks_a_reexport" />

An `__all__` built from anything other than a list or tuple of string literals, or written below module scope, holds every import in that module, as does a `from … import *`.

Two reads the count misses hold an import too. A `del` of the bound name needs that binding to exist, and a name read only inside a quoted annotation sits in a string literal rather than the tree the table walks, so each quoted annotation is parsed for the names it reads.

<Fixture rule="prune_inert_imports" case="quoted_annotation_holds_its_import" />

An import binding `__all__` itself holds on the same ground, as does a name a second import rebinds from another source, keeping the fallback in a `try: from _speedups import loads` shim standing.

A repeat of a name nothing reads takes the first binding with it, both facets resolving in the one walk rather than one per run.

<Fixture rule="prune_inert_imports" case="repeat_of_an_unread_name_drops_both_lines" />

An own-line comment directly above an import holds the whole statement, dropping the line stranding the comment on whatever follows, unless [[reflow-imports]] will fold the statement into a same-module sibling, where the drop lands on the merged line the comment then leads.

<Fixture rule="prune_inert_imports" case="leading_comment_holds_its_import" />

A package `__init__.py` reports an unreferenced import rather than dropping it, its bindings being the package's re-export surface, whereas a repeat still drops there.

## The `__future__` Directive

`from __future__ import annotations` drops wherever the directive carries no runtime weight:

1. `target-version` is 3.14 or higher, wherein PEP 749 defers evaluation.
2. No annotation runs at module scope, and every annotated name resolves to an unconditional module-scope binding written before it.

An annotation running at module scope holds the directive whatever its names resolve to, because the directive is what writes that annotation into the module's `__annotations__`, so dropping it takes a name out of the namespace the module presents. An annotation on a `def` or inside a `class` body lands on that object instead, leaving the directive free to drop.

Where [[alphabetize-siblings]] sorts definitions in the same pipeline, a name a module-level class or function binds counts as unresolved whichever side of the annotation it sits on, since the sort reseats definitions after this rule has run, so a directive covering such a reference stays in whichever order the sort writes. Where [[band-constants]] runs in the same pipeline, a binding it hoists above the annotation naming it, a constant into the leading band or an import into the import run, counts as written before that annotation, reading the module as the band seats it once the directive is gone.

<Fixture rule="composition" case="hoisted_alias_settles_the_directive" />

Every other `__future__` feature stays, `division` and its siblings changing how the module compiles rather than binding a name.

<Fixture rule="prune_inert_imports" case="division_directive_out_of_scope" />

::: tabs key:prose-target-version
== Python 3.10
The version-gated branch stays quiet, so the directive goes only where the module carries no annotation or every annotation resolves against an earlier module-scope binding.

== Python 3.11
The branch stays quiet, the same as on 3.10.

== Python 3.12
The branch stays quiet, the same as on 3.10.

== Python 3.13
The branch stays quiet, the same as on 3.10.

== Python 3.14
The version-gated branch fires, because PEP 749 lands deferred annotation evaluation and the directive carries no runtime weight.
:::

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates the `__future__` branch per project.

</template>

<template #related-after>

For the gate semantics, [**`target-version`**](/reference/configuration#top-level-keys) in the Configuration chapter covers how the field is read across version-gated rules.

</template>

</RuleLayout>

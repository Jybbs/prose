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

Both facets read the same two re-export markers, a name listed in `__all__` and the PEP 484 redundant-alias form `from x import y as y`, so a repeated self-alias survives `drop-duplicates`.

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

`from __future__ import annotations` drops on three branches, where the module carries no annotation, where `target-version` is 3.14 or higher and PEP 749 defers evaluation, and where every annotated name resolves to an unconditional module-scope binding written before it. Where [[alphabetize-siblings]] sorts definitions in the same pipeline, a name a module-level class or function binds counts as unresolved whichever side of the annotation it sits on, since the sort reseats definitions after this rule has run, so a directive covering such a reference stays in whichever order the sort writes.

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

---
caption : "Drops an import binding nothing references and a repeat of a binding an earlier import already made, reporting the one a package `__init__.py` re-exports."
lints   : true
related : [bare-imports, group-imports, modernize-annotations, single-use-variables]
layout  : doc
---

# prune-inert-imports

<RuleLayout rule="prune_inert_imports">

An import that binds a name nothing references, and a second import that rebinds a name already bound, both add a line a reader parses for no information. `prune-inert-imports` reaches either shape, dropping the unread member under `drop-unreferenced` and the repeat under `drop-duplicates`, and it resolves both against the same binding table [[single-use-variables]] reads.

<Fixture rule="prune_inert_imports" case="repeat_and_unread_member_both_go" />

The accounting runs per bound name rather than per line, so a member prunes off a shared `from` import while its siblings stay, and a line whose every member goes unread leaves with them.

<Fixture rule="prune_inert_imports" case="every_member_unread_drops_the_whole_line" />

A repeat matches on the pair of names it carries, the one it binds and the qualified path it names at its source, so `import os` beside `import os.path` is two imports of two modules rather than one repeated.

## What Holds Its Line

A name a module publishes is meant to go unread inside it, and both facets read the same two markers. A name listed in `__all__` holds its import, and so does the PEP 484 redundant-alias form `from x import y as y`, which is why a repeated self-alias survives `drop-duplicates` rather than dropping as a repeat.

<Fixture rule="prune_inert_imports" case="self_alias_marks_a_reexport" />

An `__all__` built from anything other than a list or tuple of string literals leaves the public surface unsettled, and every import in that module holds, as does one written anywhere below module scope. A `from … import *` holds too, binding a name set no reference count enumerates.

Two other reads keep an import the reference count alone would call unused. A `del` of the bound name needs the binding to exist, so removing the import would leave the statement raising `NameError`. And a name read only inside a quoted annotation sits in a string literal rather than in the tree the binding table walks, so the rule parses each quoted annotation for the names it reads and holds their imports, following the nesting where one quoted member encloses another.

<Fixture rule="prune_inert_imports" case="quoted_annotation_holds_its_import" />

An import binding `__all__` itself sets the whole export surface in one line, so it holds on the same ground a listed name does. A name a second import rebinds holds as well, because the module-scope binding then carries more than one write, and the extension shim that pairs `try: from _speedups import loads` with a pure-Python import above it needs that earlier binding on the branch the `ImportError` takes.

An own-line comment sitting directly above an import holds the whole statement too. Dropping the line would strand the comment on whatever statement follows, where it reads as a description of code it was never written about, and a comment in that position is often the record of why the import is load-bearing despite binding a name nothing reads.

<Fixture rule="prune_inert_imports" case="leading_comment_holds_its_import" />

A package `__init__.py` is the one file where an unreferenced import is reported rather than dropped, since the names it binds are the package's re-export surface and no single-file pass settles whether the package itself is what reads them. A repeat still drops there, resolving out of `sys.modules` without running its module again.

## The `__future__` Directive

`from __future__ import annotations` drops on any of three branches, when the module carries no annotation at all, when `target-version` is 3.14 or higher and PEP 749 defers annotation evaluation, or when every name every annotation reads resolves to an unconditional module-scope binding written before it.

Every other `__future__` feature stays. A directive such as `division` changes how the module compiles rather than binding a name a reference count can settle.

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

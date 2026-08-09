---
caption : "Rewrites `Optional[T]`, `Union[X, Y]`, and the `typing` generics to the `T | None`, `X | Y`, and builtin forms the target runtime carries."
related : [prune-inert-imports]
layout  : doc
---

# modernize-annotations

<RuleLayout rule="modernize_annotations">

`modernize-annotations` moves an annotation onto the spelling the language absorbed, PEP 585 giving the `typing` generics their builtin form on Python 3.9 and PEP 604 giving the unions their `|` form on 3.10, and it drops the `typing` import a module carried only to reach them.

Each rewrite runs behind its own facet and its own version floor, so a project on 3.9 converts its generics while its unions wait for 3.10. A project that has set no `target-version` at all holds both, since an unset field clears neither floor.

## What Each Facet Reaches

`rewrite-generics` converts the `typing` generics whose PEP 585 replacement is a builtin, covering `Dict`, `FrozenSet`, `List`, `Set`, `Tuple`, and `Type`. A generic whose replacement lives under `collections` instead (*`Deque`, `DefaultDict`*) stays as written, because reaching it would mean adding an import this rewrite does not add.

`rewrite-unions` joins the members of an `Optional` or a `Union` with `|`, appending the `| None` arm an `Optional` implies. A member that cannot carry the operator holds the whole annotation back, which covers a forward-reference string such as `Optional["Node"]`, where the rewritten `"Node" | None` would raise at evaluation time.

Both facets resolve their target through whatever name the module bound, so a bare `Optional`, a module-qualified `typing.Optional`, an aliased `Optional as Opt`, and the `typing_extensions` spelling of any of them all reach the same rewrite.

## Dropping the Import

A `typing` name the rewrite read out entirely leaves its import unread, and the rule drops it in the same pass, per name rather than per line:

<Fixture rule="modernize_annotations" case="import_keeps_its_surviving_names" />

A read the rewrite could not consume keeps the import standing, so a suppressed line or a held forward reference holds it. The [**Suppression**](/usage/suppression) chapter covers the directives.

::: tabs key:prose-target-version
== Python 3.10
Both facets fire, so an annotation carrying both legacy spellings settles in one pass.

== Python 3.11
Both facets fire, the same as on 3.10.

== Python 3.12
Both facets fire, the same as on 3.10.

== Python 3.13
Both facets fire, the same as on 3.10.

== Python 3.14
Both facets fire, pairing naturally with the deferred-annotation runtime that [[prune-inert-imports]] reads on the same axis.
:::

Below 3.10 only `rewrite-generics` fires, since `X | Y` raises at runtime before the PEP 604 form lands, and below 3.9 neither one does.

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates each facet per project, and an unset field holds both.

</template>

<template #related-after>

For the gate semantics, [**`target-version`**](/reference/configuration#top-level-keys) in the Configuration chapter covers how the field is read across version-gated rules.

</template>

</RuleLayout>

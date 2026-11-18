---
caption : "Rewrites `Optional[T]`, `Union[X, Y]`, and the `typing` generics to the `T | None`, `X | Y`, and builtin forms the target runtime supports."
related : [prune-inert-imports]
layout  : doc
---

# modernize-annotations

<RuleLayout rule="modernize_annotations">

`modernize-annotations` rewrites a legacy `typing` annotation to the spelling the language now supports, so `List[int]` reads `list[int]` and `Optional[int]` reads `int | None`, and it removes the `typing` import a module kept only to reach the old spellings. PEP 585 gave the `typing` generics their builtin form, and PEP 604 gave the unions their `|` form.

Each rewrite runs behind its own facet and its own version floor, read from [`target-version`](/reference/configuration#top-level-keys), so `rewrite-generics` runs from 3.9 and `rewrite-unions` from 3.10, since `X | Y` raises at runtime before PEP 604 arrives. A project with no `target-version` set keeps both spellings, since an unset field meets neither version floor.

## Dropping the Import

The rewrite leaves a `typing` import unread once it has removed every read of that name, and the rule removes that import in the same pass, one name at a time rather than one line at a time:

<Fixture rule="modernize_annotations" case="import_keeps_its_surviving_names" />

A read the rewrite could not remove keeps the import in place, so a suppressed line or a forward reference the rule left alone keeps it.

<template #facets>

Both facets find their target through whatever name the module bound, so a bare `Optional`, a module-qualified `typing.Optional`, an aliased `Optional as Opt`, and the `typing_extensions` spelling of any of them all take the same rewrite.

### `rewrite-generics`

`rewrite-generics` converts the `typing` generics whose PEP 585 replacement is a builtin, covering `Dict`, `FrozenSet`, `List`, `Set`, `Tuple`, and `Type`, with or without a subscript. A generic whose replacement lives under `collections` instead (*`Deque`, `DefaultDict`*) stays as written, because the rewrite would need an import this rule never adds.

### `rewrite-unions`

`rewrite-unions` joins the members of an `Optional` or a `Union` with `|`, appending the `| None` member an `Optional` implies. A member that cannot take the operator keeps the whole annotation in its legacy form, which covers a forward-reference string such as `Optional["Node"]`, where the rewritten `"Node" | None` would raise when the annotation is evaluated. An annotation with a comment inside its subscript stays as written too, because the rewrite rebuilds the expression and would drop the comment.

</template>

</RuleLayout>

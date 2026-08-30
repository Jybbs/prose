---
caption : "Surfaces a module-level constant whose name is not `SCREAMING_CASE`."
related : [band-constants, reassigned-constants, single-use-variables]
layout  : doc
---

# miscased-constants

<RuleLayout rule="miscased_constants">

`miscased-constants` surfaces a module-level assignment binding a fixed value under a name PEP 8 would write in `SCREAMING_CASE`, so a public global reading as `max_retries` stops implying mutable state where nothing writes it again. It completes the pair [[reassigned-constants]] opened, one rule per half of the casing contract, and it reports without rewriting, because renaming a module constant breaks every importer outside the file.

A name draws the report when it runs longer than one character, carries no leading underscore, is not already `SCREAMING_CASE`, and binds a value the module never reassigns. Five shapes stay quiet. A call-produced global is effectful (*`logger = get_logger(__name__)`, `app = build()`*), a leading underscore marks deliberate module-private state (*`_cache = {}`*), a dunder such as `__version__` takes the same exemption, a single-character name reads as a matrix or a scalar by mathematical convention, and a lambda binds a callable. Notebooks are skipped whole, a cell's top-level assignments being working variables.

A type alias is never renamed, `SCREAMING_CASE` being the one casing an alias must not take. The rule tells the two apart by the value rather than the name, so a value pointing at something already built is an alias (*`Pen = Turtle`, `open = TarFile.open`, `Interval = Union[int, float]`*), with PEP 604's `Interval = int | float` reading the same way down both sides. A value that builds something new is a constant and still draws the rename, covering a literal, an f-string, a collection display, and an arithmetic expression.

A subscript can be either, as `SETTINGS["db"]` and `Literal["read"]` both are, so three signals settle it. A slice, a dunder, or an unwrapped integer, bool, or bytes marks the base as data rather than a type, leaving `Literal[1, 2, 3]` and every `Annotated` argument past the first alone. A base found assigned a `{...}` literal in the same file marks a lookup, whereas an imported `NDArray[float]` stays a type. A name the module truth-tests, order-compares, or does arithmetic on holds data, whereas a name used in an annotation is a type whatever else reaches it. None of the three can turn a constant into an alias, so an unresolved value keeps its name and draws no report, and `database = SETTINGS["db"]` goes unflagged wherever `SETTINGS` arrives from another module.

<template #configuration>

<RuleConfigTable />

The `allow-pattern` glob is empty by default, exempting nothing beyond the structural carve-outs. A project holding a name out of `SCREAMING_CASE` on purpose sets it to spare that name, which is a different job from never flagging a type alias, and only the second happens without configuration.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[miscased-constants]` directive.

</template>

</RuleLayout>

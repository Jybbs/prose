---
caption : "Reports a module-level constant whose name is not `SCREAMING_CASE` and suggests the renamed form."
related : [band-constants, inlinable-bindings, reassigned-constants]
layout  : doc
---

# miscased-constants

<RuleLayout rule="miscased_constants">

`miscased-constants` reports a module-level assignment that binds a fixed value under a name PEP 8 would write in `SCREAMING_CASE`, and suggests that form as a display-only rename, so a public global written as `max_retries` stops reading as mutable state where nothing writes it again. It is the other half of the pair [[reassigned-constants]] opens, one rule per side of the casing contract, and it reports without rewriting, because renaming a module constant breaks every importer outside the file.

A name draws the report when it is longer than one character, has no leading underscore, is not already `SCREAMING_CASE`, and binds a value the module never reassigns. Several kinds of binding stay quiet:

1. A global produced by a call is effectful rather than fixed (*`logger = get_logger(__name__)`, `app = build()`*).
2. A leading underscore marks deliberate module-private state (*`_cache = {}`*).
3. A dunder such as `__version__` takes the same exemption.
4. A single-character name reads as a matrix or a scalar by mathematical convention.
5. A lambda binds a callable.
6. A binding inside an `if TYPE_CHECKING:` block is declared for the type checker alone.

A notebook is skipped whole, because a cell's top-level assignments are working variables.

A type alias is never renamed, because `SCREAMING_CASE` is the one casing an alias must not take, and a target annotated `TypeAlias` is exempt outright. Otherwise the rule tells an alias from a constant by the value rather than the name, so a value that points at something already built is an alias (*`Pen = Turtle`, `open = TarFile.open`, `Interval = Union[int, float]`*), and PEP 604's `Interval = int | float` reads the same way on both sides of the `|`. A value that builds something new is a constant and still draws the rename, covering a literal, an f-string, a collection display, and an arithmetic expression.

A subscript can be a constant or an alias, as `SETTINGS["db"]` and `Literal["read"]` show, so three checks settle it. A slice, a dunder, or an unwrapped integer, bool, or bytes marks the base as data rather than a type, leaving `Literal[1, 2, 3]` and every `Annotated` argument past the first alone. A base assigned a `{...}` literal in the same file marks a lookup, whereas an imported `NDArray[float]` stays a type. A name the module truth-tests, order-compares, or does arithmetic on binds data, whereas a name used in an annotation is a type whatever else reads it. None of the three checks can turn a constant into an alias, so an unresolved value keeps its name and draws no report, and `database = SETTINGS["db"]` goes unreported wherever `SETTINGS` comes from another module.

<template #configuration>

<RuleConfigTable />

The `allow-pattern` glob is empty by default and exempts nothing beyond the structural carve-outs above. A project that keeps a name out of `SCREAMING_CASE` on purpose sets the pattern to spare that name, which is a different job from never renaming a type alias, and only the second happens without configuration.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[miscased-constants]` directive.

</template>

</RuleLayout>

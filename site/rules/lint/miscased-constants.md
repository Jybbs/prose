---
caption : "Surfaces a module-level constant whose name is not `SCREAMING_CASE`."
related : [band-constants, reassigned-constants, single-use-variables]
layout  : doc
---

# miscased-constants

<RuleLayout rule="miscased_constants">

PEP 8 writes a module constant in `SCREAMING_CASE`, so a public module-level name that binds a fixed value yet reads as `max_retries` tells every caller *"mutable state"* when nothing ever writes it again. `miscased-constants` completes the pair [[reassigned-constants]] opened, one policing each half of the casing contract. It surfaces a single-name assignment whose value is inert and whose name has more than one character, carries no leading underscore, and is not already `SCREAMING_CASE`, when nothing in the module reassigns it.

The condition reads inertness through the same classifier the notebook banding gate consumes, which is what keeps the lint quiet where pylint's equivalent drowns in noise. A call-produced global (*`logger = get_logger(__name__)`, `app = build()`*) is effectful, so it never flags, and a leading underscore marks deliberate module-private state (*`_cache = {}`*), the dunder `__version__` riding the same exemption. A single-character name is spared too, its lone-capital `SCREAMING` form reading as a matrix by linear-algebra convention and its lowercase form usually a mathematical scalar. What remains is public data with no reassignment anywhere in the module, exactly the shape the `SCREAMING_CASE` convention exists for. The lint reports without an auto-fix and names the `SCREAMING_CASE` form in its help line, because renaming a module constant breaks importers outside the file. Notebooks are skipped whole, a cell's top-level assignments being working variables rather than module constants.

A bare PEP 484 type alias is spared on the shape of its value rather than the casing of its target, because `SCREAMING_CASE` is the one rename an alias must never take, PascalCase being what an alias is written in. A value that names an object already built (*`Pen = Turtle`, `open = TarFile.open`, `Interval = Union[int, float]`*) is an alias, and PEP 604's `Interval = int | float` reads the same way, recursing into both sides so that `1 | 2` stays a constant. A value that constructs new data (*a literal, an f-string, a collection display, an arithmetic expression*) is a constant and still draws the rename, which keeps every dispatch table and derived string the lint exists to catch. A lambda binds a callable rather than data, so it is spared too. The same read decides the alias sub-band in [[band-constants]], so one answer serves both rules.

<template #configuration>

<RuleConfigTable />

The `allow-pattern` regex is empty by default, exempting nothing beyond the structural carve-outs. A project holding a name out of `SCREAMING_CASE` on purpose sets it to spare that name, which is a different job from never flagging a type alias, and only the second happens without configuration.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[miscased-constants]` directive.

</template>

</RuleLayout>

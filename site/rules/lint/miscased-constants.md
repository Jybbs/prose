---
caption : "Surfaces a module-level constant whose name is not `SCREAMING_CASE`."
related : [band-constants, reassigned-constants, single-use-variables]
layout  : doc
---

# miscased-constants

<RuleLayout rule="miscased_constants">

PEP 8 writes a module constant in `SCREAMING_CASE`, so a public module-level name that binds a fixed value yet reads as `max_retries` tells every caller *"mutable state"* when nothing ever writes it again. `miscased-constants` completes the pair [[reassigned-constants]] opened, one policing each half of the casing contract. It surfaces a single-name assignment whose value is inert and whose name has more than one character, carries no leading underscore, and is not already `SCREAMING_CASE`, when nothing in the module reassigns it.

The condition reads inertness through the same classifier the notebook banding gate consumes, which is what keeps the lint quiet where pylint's equivalent drowns in noise. A call-produced global (*`logger = get_logger(__name__)`, `app = build()`*) is effectful, so it never flags, and a leading underscore marks deliberate module-private state (*`_cache = {}`*), the dunder `__version__` riding the same exemption. A single-character name is spared too, its lone-capital `SCREAMING` form reading as a matrix by linear-algebra convention and its lowercase form usually a mathematical scalar. What remains is public data with no reassignment anywhere in the module, exactly the shape the `SCREAMING_CASE` convention exists for. The lint reports without an auto-fix and names the `SCREAMING_CASE` form in its help line, because renaming a module constant breaks importers outside the file. Notebooks are skipped whole, a cell's top-level assignments being working variables rather than module constants.

A type alias is never renamed, because `SCREAMING_CASE` is the one casing an alias must not take, PascalCase being what an alias is written in. The rule tells an alias from a constant by looking at the value rather than at the name. A value that points at something already built (*`Pen = Turtle`, `open = TarFile.open`, `Interval = Union[int, float]`*) is an alias, and PEP 604's `Interval = int | float` reads the same way, recursing into both sides so that `1 | 2` stays a constant. A value that builds something new (*a literal, an f-string, a collection display, an arithmetic expression*) is a constant and still draws the rename, which is what keeps the dispatch tables and derived strings the lint exists to catch. A lambda binds a callable, so it is spared too.

Looking at the value is not always enough, since `SETTINGS["db"]` and `Literal["read"]` are the same shape. Two further checks run, and none of the three can ever turn a constant into an alias, only an alias into a constant, so a value that none of them pins down keeps its name and draws no warning. That is deliberate, in that a missed warning costs one line of output whereas renaming a real alias breaks every module that imports it.

The first checks the slice against what a type is allowed to contain. A type never holds a slice or a dunder, which makes `path_separators[1:]` and `sys.modules[__name__]` lookups into data, and it never holds an integer, signed or not, a bool, or bytes outside `Literal`, which makes `LEVELS[1]` and `items[-1]` indexes while `Literal[1, 2, 3]` and `Literal[-1]` stay types. PEP 593 lets every `Annotated` argument after the first be anything, so the call in `Annotated[int, Field(gt=0)]` is metadata rather than a call in a type position. The second looks the base up in the module's own bindings, so `SETTINGS["db"]` is a dict lookup once `SETTINGS` is found assigned a `{...}` literal in the same file, whereas an imported `NDArray[float]` and a locally declared `Box[int]` are both types. The third reads how the module uses the name, so a name it truth-tests, order-compares, or does arithmetic on holds data (*`if path_sep:`*), whereas a name used in an annotation is a type whatever else happens to it.

What the third check ignores matters as much as what it counts, because a class does most of what data does. An `Enum` is iterable, so `for color in Colors:` says nothing about `Colors`. A class compares by equality, so `type(value) == Kind` says nothing about `Kind`, and typing-aware code compares type objects with `is` (*`if base is Generic:`*) and reads a class alias for its methods (*`int_.from_bytes`*). What counts as data is the short list of operations a class raises `TypeError` on.

The same caution sets the rule's limit. When the base cannot be resolved in the file, because it was imported or bound by a call, the subscript stays a type, so `database = SETTINGS["db"]` goes unflagged where `SETTINGS` comes from elsewhere. Settling that would take a cross-file import map, which is exactly the machinery this rule does without.

The value read is what [[band-constants]] consumes for its alias sub-band, so one answer settles both the rename and the band. The read contexts stay with the lint, a name being a type wherever it is used whereas the band sorts on the value alone.

<template #configuration>

<RuleConfigTable />

The `allow-pattern` regex is empty by default, exempting nothing beyond the structural carve-outs. A project holding a name out of `SCREAMING_CASE` on purpose sets it to spare that name, which is a different job from never flagging a type alias, and only the second happens without configuration.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[miscased-constants]` directive.

</template>

</RuleLayout>

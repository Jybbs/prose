---
caption : "Reports a function-local binding written once and read once whose value inlines at its read at no cost."
related : [line-overflow, reassigned-constants, step-narration]
layout  : doc
---

# inlinable-bindings

<RuleLayout rule="inlinable_bindings">

`inlinable-bindings` reports a function-local binding that is written once and read once when its value would drop into that single read at no cost, leaving the same computation in the same place on a row that still fits. A binding written once and read once is not a defect on its own, because naming an intermediate is the ordinary way an author gives a reader a handle, so the rule reports only that narrower case and leaves the inline-or-keep decision to whoever reads the finding.

The rule reads the per-`Source` [[binding-analysis]] table to count writes and reads per binding, then drops every candidate whose inline would cost something:

- A value that already spans rows stays as written, because the replacement would carry those rows into the read.
- A read inside a region the write sits outside of stays as written, covering a loop body, a `while` test, a `try` or `with` arm, an `except` clause naming the exception class, a nested function, a lambda body, and the per-item part of a comprehension, because the inline would change how often the value is computed, what guards it, or what a closure captures.
- A swap that pushes the read's own row past `code-line-length` stays as written, because the layout rules would then break the call across rows and the file would grow for a name removed.
- A candidate for which no replacement text resolves is withheld rather than reported bare, so every finding names the expression that would stand in the binding's place.

Those regions exempt only the read that sits inside them, so a part of the same construct that runs once, where the author wrote it, keeps its finding, and a comprehension's outermost iterable and a lambda's parameter default are both still reported.

Several kinds of binding never reach that cost test at all:

- A binding matching the `allow-pattern` glob (*default `_*`, which exempts an intentionally-unused name*) stays quiet, as does a binding a later `del` names, because the inline would leave that `del` naming a value nothing bound.
- An augmented assignment counts as both a write and a read, so a binding it targets reaches two uses.
- A loop variable, a comprehension target, and a function parameter are bound implicitly and stay outside the rule's reach.
- A walrus expression's own value counts as a use, so a walrus target reaches two uses wherever anything consumes it.
- A function that declares `global` or `nonlocal` anywhere in its body is skipped whole.
- A tuple-unpack target stays exempt when a sibling target reads more than once. Where every target reads once, the diagnostic names the subscript rewrite (*`batch[0]` for the first target of `x, y = batch`*) whenever the right-hand side is a plain name or attribute.

The lint never rewrites, so the diagnostic is reported and the source stays as written.

<template #configuration>

<RuleConfigTable />

The default `_*` exempts names starting with an underscore, the Python convention for an intentionally-unused binding. A project with stricter naming can tighten the glob, and an empty pattern exempts nothing rather than everything, the same reading [[miscased-constants]] gives its own empty default.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[inlinable-bindings]` directive.

</template>

</RuleLayout>

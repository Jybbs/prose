---
caption : "Surfaces a local binding whose value inlines at its read for free."
related : [reassigned-constants, step-narration]
layout  : doc
---

# inlinable-bindings

<RuleLayout rule="inlinable_bindings">

A binding written once and read once is not a defect on its own, because naming an intermediate is the ordinary way an author hands a reader a handle. The condition worth flagging is narrower, being the binding whose value would drop into its single read at no cost, leaving the same computation in the same place on a row that still fits. `inlinable-bindings` reports exactly that binding and leaves the inline-or-keep decision to whoever reads the finding.

The rule consumes the per-`Source` [[binding-analysis]] table to count writes and reads per binding, then declines every candidate whose inline would cost something. A value that already spans rows stays put, since the replacement would carry those rows into the read. A read sitting inside a loop body, a `try` or `with` arm, a nested function, or the per-item part of a comprehension the write sits outside of stays put, since the inline would change how often the value is computed, what guards it, or what a closure captures. The parts of those constructs that run once where the author wrote them keep their finding, so a comprehension's outermost iterable and a lambda's parameter default are both still reported. A swap that carries the read's own row past `code-line-length` stays put, since the layout rules would then break the call across rows and the file would grow for a name removed. Every surviving finding names the expression that would stand in the binding's place, so a candidate resolving no replacement text is withheld rather than reported bare.

Bindings matching the `allow-pattern` glob (*defaulting to `_*`, which exempts intentionally-unused names*) stay quiet. Augmented assignments count as both a write and a read, so a binding they target reaches two uses. Loop variables, comprehension targets, and function parameters are introduced implicitly and stay outside the rule's reach. A walrus expression's own value counts as a use, so a walrus target reaches two uses wherever anything consumes it. A tuple-unpack target stays exempt when a sibling target reads more than once, and where every target reads once the diagnostic names the subscript rewrite (*`batch[0]` for the first target of `x, y = batch`*) whenever the right-hand side is a plain name or attribute. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

<RuleConfigTable />

The default `_*` exempts names starting with an underscore, matching the Python convention for intentionally-unused bindings. Projects with stricter naming can tighten the glob, and an empty pattern reads as exempting nothing rather than everything, the same reading [[miscased-constants]] gives its own empty default.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[inlinable-bindings]` directive.

</template>

</RuleLayout>

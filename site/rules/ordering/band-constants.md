---
caption : "Hoists module-level constants into a leading band below the imports and a trailing band beneath the definitions."
related : [alphabetize-siblings, group-imports, space-statements, align-equals, miscased-constants, reassigned-constants]
layout  : doc
---

# band-constants

<RuleLayout rule="band_constants">

`band-constants` gathers module-level constants into two bands, a leading band directly below the imports and a trailing band beneath the definitions, so a module reads top to bottom as its imports, its leading constants, its definitions, then the constants derived from them.

| Band | Holds |
|---|---|
| Leading | a constant whose value reaches only imports, builtins, literals, or fellow leading constants |
| Trailing | a constant that names a function or class defined later in the module |

The rule relocates a constant into its band, and each band orders by `(tier, subcategory, name)`, clustering the type aliases ahead of the `SCREAMING_CASE` constants and those ahead of the remaining module state. A constant reading another band member climbs an evaluation tier, and each tier opens its own blank-separated sub-band so derived values read apart from the primitives they build on. A tier holding a single constant folds tight below the tier above and aligns with it through [[align-equals]]. `max-tiers` caps how many tiers open a sub-band.

Only an evaluation-time reference binds the order, covering a right-hand side, a decorator, a default argument, a base class, and a non-deferred annotation, so a constant a function reads inside its body still joins the leading band. Several shapes pin a constant where the author left it:

- A reassigned name.
- A value naming an unresolved reference.
- A line under a suppression directive or a `# prose: keep` marker.
- An import a trailing `noqa` comment marks, either bare or naming `E402`, the code the wider ecosystem reports a late import under.
- A row a `\` line join continues.
- Every constant in a reference cycle.

A constant also pins wherever banding it would change which object a name resolves to while the module runs. That covers:

1. A constant whose own name shadows a builtin some definition above it already read.
2. A value reaching through an attribute or a subscript into a name a definition above it reads at evaluation time.
3. A value resolving a name against a builtin or an earlier module-scope write that a definition below it rebinds, where a write inside a branch, an import a guard wraps, and a `global` write from a call the module makes each count as that earlier binding.

Each case resolves one object before the move and a different one after, without raising, so the constant holds its slot instead.

A statement reading a dunder the module later rebinds holds the whole region in source order, because the loader binds every module dunder before the body runs, so seating the rebind above the read would hand it the new value. Every other name is unbound until its own statement runs, leaving a hoist above a reader able only to resolve a reference rather than to change one.

Only an inert value bands. An inert value reads names and builds a result (*a literal, a name, an attribute or subscript read, a display or operator expression, or a `lambda`*), whereas an effectful value carries a call, a comprehension, or an `await` and moving it would reorder that work. `RANDOM_SEED = 42` hoists into the leading band whereas `wide_trainer = L.Trainer(**trainer_kwargs)` holds its place.

An own-line comment above a member travels with it wherever the rule seats it, and a comment on the line below documents that member instead and travels the other way. A banner (*`# --- Configuration ---`*), a suppression directive, a tool pragma (*`# noqa`*), and a comment opening at another indent each hold their slot and pin the member beneath, so a band never crosses a banner. A notebook carries the same reach as a module, with its cell boundary bounding the carry.

Relocation and its spacing settle in one run, so the file reaches its final shape on the first format.

<FixtureConvergence rule="band_constants" case="stacked_comment_blocks_keep_their_blank" />

<template #configuration>

<RuleConfigTable />

The facets above tune the band without switching it off. `group-subcategories` clusters each band by subcategory, dropping to a plain `(tier, name)` sort when `false`. `max-tiers` caps how many evaluation tiers open their own sub-band, defaulting to `2` so a band reads as its base plus one derived sub-band, with `1` holding it tight and `false` opening one per tier. Turned off entirely with `band-constants = false`, the constants stay in place among their neighbors. The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* decides which imports the leading band seats below, since a first-party package's imports group with the local-package section.

</template>

</RuleLayout>

---
caption : "Moves module-level constants into a leading band below the imports and a trailing band below the definitions, and sorts each band."
related : [alphabetize-siblings, group-imports, space-statements, align-equals, miscased-constants, reassigned-constants]
layout  : doc
---

# band-constants

<RuleLayout rule="band_constants">

`band-constants` moves module-level constants into two bands and sorts each, a leading band directly below the imports and a trailing band below the definitions, so a module reads top to bottom as its imports, its leading constants, its definitions, then the constants derived from them.

| Band | Members |
|---|---|
| Leading | a constant whose value reads only imports, builtins, literals, or other leading constants |
| Trailing | a constant that names a function or class defined later in the module |

The rule moves a constant into its band, and each band sorts by `(tier, subcategory, name)`, clustering the type aliases ahead of the `SCREAMING_CASE` constants and those ahead of the remaining module state. A constant that reads another band member climbs one evaluation tier, and each tier opens its own blank-separated sub-band, so derived values read apart from the primitives they build on. A tier with a single constant sits tight below the tier above and aligns with it through [[align-equals]]. `max-tiers` caps how many tiers open a sub-band.

A band carries its own order, whereas [[group-imports]] moves an import into its section and leaves the order within it to [[alphabetize-siblings]]. The split follows what each order costs to get wrong, in that import siblings reorder freely whereas a constant's slot binds every reference to it, so the move is only safe under the evaluation analysis this rule already runs.

Only an evaluation-time reference binds the order, covering a right-hand side, a decorator, a default argument, a base class, and a non-deferred annotation, so a constant a function reads inside its body still joins the leading band. Several cases pin a constant where the author left it:

- A reassigned name.
- A value naming an unresolved reference.
- A line under a suppression directive or a `# prose: keep` marker.
- An import a trailing `noqa` comment marks, either bare or naming `E402`, the code the wider ecosystem reports a late import under.
- A row a `\` line join continues.
- Every constant in a reference cycle.

A constant also stays put wherever moving it would change which object a name resolves to while the module runs. That covers:

1. A constant whose own name shadows a builtin some definition above it already reads.
2. A value reaching through an attribute or a subscript into a name a definition above it reads at evaluation time.
3. A value resolving a name against a builtin or an earlier module-scope write that a definition below it rebinds, where a write inside a branch, an import inside a guard, and a `global` write from a call the module makes each count as that earlier binding.

Each case resolves one object before the move and a different one after, without raising, so the constant keeps its slot instead.

A statement reading a dunder the module later rebinds keeps the whole region in source order, because the loader binds every module dunder before the body runs, so placing the rebind above the read would give it the new value. Every other name is unbound until its own statement runs, so a move above a reader can only resolve a reference, never change one.

Only an inert value bands, meaning one that reads names and builds a result (*a literal, a name, an attribute or subscript read, a display or operator expression, or a `lambda`*), whereas an effectful value carries a call, a comprehension, or an `await`, and moving it would reorder that work. `RANDOM_SEED = 42` moves into the leading band whereas `wide_trainer = L.Trainer(**trainer_kwargs)` stays where it is.

A constant the analysis pins for a reassigned or unresolved name, a resolution hazard, or an effectful value still spaces as a member of the band beside it, so its pair with a banded constant sits tight, a tier boundary between them opens one blank line, and a heading standing a blank line above the pinned constant keeps one blank line above it. Every other pinned member keeps the gap the source wrote.

An own-line comment above a member travels with it wherever the rule places it, and a comment on the line directly below a member documents that member instead and follows it rather than leading it. A banner (*`# --- Configuration ---`*), a suppression directive, a tool pragma (*`# noqa`*), and a comment at another indent each keep their slot and pin the member beneath, so a band never crosses a banner. A notebook has the same reach as a module, with each cell boundary bounding the move.

The move and its spacing settle in one run, so the file reaches its final layout on the first format.

<FixtureConvergence rule="band_constants" case="stacked_comment_blocks_keep_their_blank" />

<template #configuration>

<RuleConfigTable />

The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* sets which imports the leading band sits below, since a first-party package's imports group with the local-package section.

</template>

<template #facets>

The facets below tune the band without switching it off, whereas `band-constants = false` leaves every constant in place among its neighbors.

### `group-subcategories`

`group-subcategories` clusters each band by subcategory, putting the type aliases ahead of the `SCREAMING_CASE` constants and those ahead of the remaining module state. Setting it to `false` sorts on `(tier, name)` alone, so each tier reads as one alphabetical run.

### `max-tiers`

`max-tiers` caps how many evaluation tiers open their own blank-separated sub-band, merging every deeper tier into the last. It defaults to <ConfigDefault rule="band-constants" facet="max-tiers" /> so a band reads as its base plus one derived sub-band, where `1` keeps the whole band together and `false` gives every tier a sub-band of its own.

</template>

</RuleLayout>

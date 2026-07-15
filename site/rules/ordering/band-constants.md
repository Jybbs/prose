---
caption : "Hoists module-level constants into a leading band below the imports and a trailing band beneath the definitions."
related : [alphabetize, group-imports, blank-lines, align-equals, miscased-constants, reassigned-constants]
layout  : doc
---

# band-constants

<RuleLayout rule="band_constants">

A reader opening a module wants its shape to declare itself: what it draws on, what it defines, and the values that fall out of those definitions. When constants sit wherever they were typed, that shape blurs, a configuration value buried between two functions reading no differently from a table derived from them. `band-constants` **gathers module-level constants into two bands**, a leading band directly below the imports and a trailing band beneath the definitions, so a module reads top to bottom as its imports, its leading constants, its definitions, then the constants derived from them.

| Band | Holds |
|---|---|
| **Leading** | a constant whose value reaches only imports, builtins, literals, or fellow leading constants |
| **Trailing** | a constant that names a function or class defined later in the module |

The rule **relocates** a constant into its band, the move that makes the banding a structural concern rather than an alphabetizing one. Each band sorts within its dependency tier by `(tier, subcategory, name)`, clustering the **type aliases** ahead of the **`SCREAMING_CASE` constants** and those ahead of the remaining **module state**, so like kinds sit together and a constant another constant reads stays above its reader. The tiering and soundness analysis it rests on lives in the shared `tiering` primitive that [[alphabetize]] reads for its definition runs too.

A constant that reads another band member climbs an **evaluation tier**, and each tier opens its own blank-separated sub-band, so a module's derived values read apart from the primitives they build on. A tier holding a single constant is the exception, folding tight below the tier above and aligning with it through [[align-equals]] rather than standing alone across a blank line, the way a one-element collection stays inline. The `max-tiers` facet caps how many tiers open their own sub-band, and `group-constants` gates the subcategory clustering.

Only an **evaluation-time reference** binds the order, a right-hand side, a decorator, a default argument, a base class, or a non-deferred annotation. A constant a function reads only inside its body still rides the leading band, because the body does not run at import time. A constant the rule cannot place safely *(a reassigned name, a value naming an unresolved reference, or a line a suppression directive or a `# prose: keep` marker covers)* pins where the author left it, and a reference graph that forms a cycle leaves every constant in place.

Inside a **notebook** cell the rule narrows further, banding a constant only when its value is **inert** and holding an **effectful** value in place. An inert value merely reads names and builds a result *(a literal, a name, an attribute or subscript read, a display or operator expression over these, or a `lambda`, whose body does not run at binding)*, so relocating it changes nothing a later cell observes. An effectful value carries a call, a comprehension, or an `await` somewhere in its expression tree, so evaluating it runs code beyond reading names, and moving it reorders that work against the cell's other statements. A cell assigning `RANDOM_SEED = 42` still hoists the seed into the leading band, whereas `wide_trainer = L.Trainer(**trainer_kwargs)` holds its place, because a notebook cell runs top to bottom and a seeded run draws its random numbers in that order. A `.py` module keeps the broader reach, where a relocated value is typically a compiled pattern or a lookup table rather than a step in a running program.

An own-line comment above a constant rides into the band with it, so a note stays attached to the value it describes. A decorative banner *(`# --- Configuration ---`)* is the exception, reading as a section divider that pins the constant beneath it, and a band never crosses such a marker into the section above it.

[[group-imports]] sections the import run before `band-constants` seats the leading band beneath it, and [[blank-lines]] settles the spacing around the definitions, leaving `band-constants` to own the single blank line dividing the imports from the leading band.

Pair with [[alphabetize]] to sort the names within each import section and the definition runs, with [[group-imports]] to partition the imports the leading band seats below, and with [[reassigned-constants]] to flag a `SCREAMING_CASE` name whose reassignment pins it out of a band.

<template #configuration>

<RuleConfigTable />

`band-constants` carries the `group-constants` and `max-tiers` facets beyond its `enabled` toggle. `group-constants` clusters each band by subcategory, dropping to a plain `(tier, name)` sort when `false`. `max-tiers` caps how many evaluation tiers open their own sub-band, defaulting to `2` so a band reads as its base plus one derived sub-band, with `1` holding it tight and `false` opening one per tier. Turned off entirely with `band-constants = false`, the constants stay in place among their neighbors. The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* decides which imports the leading band seats below, since a first-party package's imports group with the local-package section.

</template>

</RuleLayout>

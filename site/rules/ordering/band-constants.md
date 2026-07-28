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

Only an **evaluation-time reference** binds the order, a right-hand side, a decorator, a default argument, a base class, or a non-deferred annotation. A constant a function reads only inside its body still joins the leading band, because the body does not run at import time. A constant the rule cannot place safely *(a reassigned name, a value naming an unresolved reference, or a line a suppression directive or a `# prose: keep` marker covers)* pins where the author left it, and a reference graph that forms a cycle leaves every constant in place.

The rule bands a constant only when its value is **inert**, holding an **effectful** value in place. An inert value merely reads names and builds a result *(a literal, a name, an attribute or subscript read, a display or operator expression over these, or a `lambda`, whose body does not run at binding)*, so relocating it changes nothing a later statement observes. An effectful value carries a call, a comprehension, or an `await` somewhere in its expression tree, so evaluating it runs code beyond reading names, and moving it reorders that work against the statements around it. `RANDOM_SEED = 42` still hoists into the leading band, whereas `wide_trainer = L.Trainer(**trainer_kwargs)` holds its place, because a module runs top to bottom and a seeded run draws its random numbers in that order. A `.py` module and a **notebook** cell carry the same reach, since evaluation order is observable in both.

An own-line comment above a member travels with it wherever the rule seats it, a constant, an import, and a definition alike, so a note stays attached to whatever it describes. A comment on the line directly below a member, held off the next one by a blank line, documents the member above it instead and travels the other way, trailing that member's code where the line has room and climbing onto the line above it where it does not. A run touching both members reads as the description of the one beneath it. A decorative banner *(`# --- Configuration ---`)*, a suppression directive, a tool pragma *(`# type: ignore`, `# noqa`)*, and a comment opening at another indent than the member below it are the exceptions, each holding the slot the author gave it and pinning the member beneath it, and a band never crosses a banner into the section above it. Inside a notebook the cell boundary bounds the carry too, so a comment closing one cell stays where the author typed it while the member in the next cell bands without it.

[[group-imports]] sections the import run before `band-constants` seats the leading band beneath it, and [[blank-lines]] settles the spacing around the definitions and the single blank line dividing the imports from the leading band.

A member's relocation and the spacing around it settle in **the same run**, so the file reaches its final shape the first time it is formatted rather than advancing one structural gap per invocation. Running `prose format` again reads that output and rewrites nothing.

<FixtureConvergence rule="band_constants" case="stacked_comment_blocks_keep_their_blank" />

Pair with [[alphabetize]] to sort the names within each import section and the definition runs, with [[group-imports]] to partition the imports the leading band seats below, and with [[reassigned-constants]] to flag a `SCREAMING_CASE` name whose reassignment pins it out of a band.

<template #configuration>

<RuleConfigTable />

`band-constants` carries the `group-constants` and `max-tiers` facets beyond its `enabled` toggle. `group-constants` clusters each band by subcategory, dropping to a plain `(tier, name)` sort when `false`. `max-tiers` caps how many evaluation tiers open their own sub-band, defaulting to `2` so a band reads as its base plus one derived sub-band, with `1` holding it tight and `false` opening one per tier. Turned off entirely with `band-constants = false`, the constants stay in place among their neighbors. The `imports.first-party` list under `[imports]` *(see the [configuration reference](/reference/configuration#imports))* decides which imports the leading band seats below, since a first-party package's imports group with the local-package section.

</template>

</RuleLayout>

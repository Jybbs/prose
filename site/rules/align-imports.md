---
category : auto-fix
family   : alignment
caption  : "aligns the `import` and `as` keywords across consecutive import statements."
related  : [align-colons, align-equals, alphabetize, bare-import-allowlist, blank-lines, match-case-align]
layout   : doc
---

# align-imports

<RuleLayout rule="align_imports">

An import block carries two kinds of structure that the reader's eye wants to follow as columns. The module column says where a thing comes from, and the name column says what's pulled in. When the two columns float at varying widths, every line reads as a fresh sentence rather than a row in a table. `align-imports` gathers consecutive `from ... import ...` statements (*or consecutive `import ... as ...` statements*) into a shared column for the `import` (*or `as`*) keyword, leaving the module column flush left and the name column flush right.

The rule reads each block as the run of consecutive imports at the same indentation. A blank line, a comment, or a non-import statement resets the run. Pair with [[alphabetize]] to sort entries within each block before alignment, with [[blank-lines]] to separate import groups by category, and with [[bare-import-allowlist]] to canonicalize bare-versus-from before the alignment pass.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce. When a block's widest module name would push the `import` keyword past the cap, `max-shift-policy` decides the fallback shape, with a worked example per policy. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the full semantics.

</template>

<template #canonical-lead>

A run of `from ... import ...` statements lines up on the `import` keyword, so the module column flushes left and the name column flushes right.

</template>

</RuleLayout>
